use actix_web::{error, get, post, web, Error, HttpRequest, HttpResponse, Result};

use entity::post;
use entity::post::Entity as Post;
use sea_orm::{entity::*, query::*};
use serde::{Deserialize, Serialize};

use crate::AppState;

const DEFAULT_POSTS_PER_PAGE: u64 = 5;

#[derive(Deserialize, Serialize, Debug, Clone)]
struct FlashData {
    kind: String,
    message: String,
}

#[derive(Debug, Deserialize)]
struct Params {
    page: Option<u64>,
    posts_per_page: Option<u64>,
}

// ADMIN ROUTES

#[get("/admin/posts")]
async fn admin_posts(data: web::Data<AppState>) -> Result<HttpResponse, Error> {
    let template = &data.templates;
    let mut ctx = tera::Context::new();

    ctx.insert("page_title", "Posts");

    let body = template
        .render("admin/pages/posts.html.tera", &ctx)
        .map_err(|e| error::ErrorInternalServerError(format!("Template error: {:?}", e)))?;

    Ok(HttpResponse::Ok().content_type("text/html").body(body))
}

#[get("/admin")]
async fn admin_index(data: web::Data<AppState>) -> Result<HttpResponse, Error> {
    let template = &data.templates;
    let mut ctx = tera::Context::new();

    ctx.insert("page_title", "Dashboard");

    let body = template
        .render("admin/pages/home.html.tera", &ctx)
        .map_err(|e| error::ErrorInternalServerError(format!("Template error: {:?}", e)))?;

    Ok(HttpResponse::Ok().content_type("text/html").body(body))
}

// FRONTEND ROUTES

#[post("/posts")]
async fn create(
    data: web::Data<AppState>,
    post_form: web::Form<post::Model>,
) -> actix_flash::Response<HttpResponse, FlashData> {
    let conn = &data.conn;
    let form = post_form.into_inner();

    post::ActiveModel {
        title: Set(form.title.to_owned()),
        text: Set(form.text.to_owned()),
        ..Default::default()
    }
    .save(conn)
    .await
    .expect("could not insert post");

    let flash = FlashData {
        kind: "success".to_owned(),
        message: "Post successfully added.".to_owned(),
    };

    actix_flash::Response::with_redirect(flash, "/")
}

#[get("/posts/new")]
async fn new(data: web::Data<AppState>) -> Result<HttpResponse, Error> {
    let template = &data.templates;
    let ctx = tera::Context::new();
    let body = template
        .render("new.html.tera", &ctx)
        .map_err(|_| error::ErrorInternalServerError("Template error"))?;

    Ok(HttpResponse::Ok().content_type("text/html").body(body))
}

#[get("/posts/{id}")]
async fn edit(data: web::Data<AppState>, id: web::Path<i32>) -> Result<HttpResponse, Error> {
    let conn = &data.conn;
    let template = &data.templates;
    let post: post::Model = Post::find_by_id(id.into_inner())
        .one(conn)
        .await
        .expect("could not find the post")
        .unwrap();
    let mut ctx = tera::Context::new();
    ctx.insert("post", &post);

    let body = template
        .render("edit.html.tera", &ctx)
        .map_err(|_| error::ErrorInternalServerError("Template error"))?;

    Ok(HttpResponse::Ok().content_type("text/html").body(body))
}

#[get("/")]
async fn list(
    req: HttpRequest,
    data: web::Data<AppState>,
    opt_flash: Option<actix_flash::Message<FlashData>>,
) -> Result<HttpResponse, Error> {
    let template = &data.templates;
    let conn = &data.conn;
    let params = web::Query::<Params>::from_query(req.query_string()).unwrap();
    let page = params.page.unwrap_or(1);
    let posts_per_page = params.posts_per_page.unwrap_or(DEFAULT_POSTS_PER_PAGE);
    let paginator = Post::find()
        .order_by_asc(post::Column::Id)
        .paginate(conn, posts_per_page.try_into().unwrap());
    let num_pages = paginator.num_pages().await.ok().unwrap();
    let posts = paginator
        .fetch_page((page - 1).try_into().unwrap())
        .await
        .expect("could not retrieve posts");
    let mut ctx = tera::Context::new();

    ctx.insert("posts", &posts);
    ctx.insert("page", &page);
    ctx.insert("posts_per_page", &posts_per_page);
    ctx.insert("num_pages", &num_pages);

    if let Some(flash) = opt_flash {
        let flash_inner = flash.into_inner();
        ctx.insert("flash", &flash_inner);
    };

    let body = template
        .render("index.html.tera", &ctx)
        .map_err(|_| error::ErrorInternalServerError("Template error"))?;

    Ok(HttpResponse::Ok().content_type("text/html").body(body))
}

#[post("/posts/{id}")]
async fn update(
    data: web::Data<AppState>,
    id: web::Path<i32>,
    post_form: web::Form<post::Model>,
) -> actix_flash::Response<HttpResponse, FlashData> {
    let conn = &data.conn;
    let form = post_form.into_inner();

    post::ActiveModel {
        id: Set(id.into_inner()),
        title: Set(form.title.to_owned()),
        text: Set(form.text.to_owned()),
    }
    .save(conn)
    .await
    .expect("could not edit post");

    let flash = FlashData {
        kind: "success".to_owned(),
        message: "Post successfully updated.".to_owned(),
    };

    actix_flash::Response::with_redirect(flash, "/")
}

#[post("/posts/{id}/delete")]
async fn delete(
    data: web::Data<AppState>,
    id: web::Path<i32>,
) -> actix_flash::Response<HttpResponse, FlashData> {
    let conn = &data.conn;
    let post: post::ActiveModel = Post::find_by_id(id.into_inner())
        .one(conn)
        .await
        .unwrap()
        .unwrap()
        .into();

    post.delete(conn).await.unwrap();

    let flash = FlashData {
        kind: "success".to_owned(),
        message: "Post successfully updated.".to_owned(),
    };

    actix_flash::Response::with_redirect(flash, "/")
}
