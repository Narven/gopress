use actix_files as fs;
use actix_web::{middleware, web, App, HttpServer};

use listenfd::ListenFd;
use migration::{Migrator, MigratorTrait};
use sea_orm::DatabaseConnection;
use std::env;
use tera::Tera;

#[derive(Debug, Clone)]
struct AppState {
    templates: Tera,
    conn: DatabaseConnection,
}

mod routes;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // dotenv
    std::env::set_var("RUST_LOG", "debug");
    tracing_subscriber::fmt::init();
    dotenv::dotenv().ok();
    // env variables
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL is not set in .env file");
    let host = env::var("HOST").expect("HOST is not set in .env file");
    let port = env::var("PORT").expect("PORT is not set");
    let server_url = format!("{}:{}", host, port);
    // conn
    let conn = sea_orm::Database::connect(&db_url).await.unwrap();
    // migration
    Migrator::up(&conn, None).await.unwrap();
    let templates = Tera::new(concat!(env!("CARGO_MANIFEST_DIR"), "/templates/**/*")).unwrap();
    let state = AppState { templates, conn };
    let mut listenfd = ListenFd::from_env();

    let mut server = HttpServer::new(move || {
        App::new()
            .data(state.clone())
            .wrap(middleware::Logger::default()) // enable logger
            .wrap(actix_flash::Flash::default())
            .configure(init)
            .service(fs::Files::new("/static", "./static").show_files_listing())
    });

    server = match listenfd.take_tcp_listener(0)? {
        Some(listener) => server.listen(listener)?,
        None => server.bind(&server_url)?,
    };
    println!("Starting server at {}", server_url);
    server.run().await?;

    Ok(())
}

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(routes::list);
    cfg.service(routes::new);
    cfg.service(routes::create);
    cfg.service(routes::edit);
    cfg.service(routes::update);
    cfg.service(routes::delete);
    // admin
    cfg.service(routes::admin_index);
    cfg.service(routes::admin_posts);
}
