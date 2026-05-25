mod db;
mod models;

use actix_web::{App, HttpResponse, HttpServer, Responder, web};
use tera::{Context, Tera}; // Added Context to pass variables to HTML

async fn home_page(tera: web::Data<Tera>) -> impl Responder {
    let mut ctx = Context::new();
    ctx.insert("project_title", "Patient Management System");

    match tera.render("index.html", &ctx) {
        Ok(html_content) => HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(html_content),
        Err(e) => {
            println!("Template error: {}", e);
            HttpResponse::InternalServerError().body("Failed to compile layout.")
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // To load environment variables from .env file and initialize logger
    dotenv::dotenv().ok();
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    println!("Starting Patient Management System...");

    let db_pool = db::create_db_pool()
        .await
        .expect("Failed to create database pool");
    sqlx::migrate!()
        .run(&db_pool)
        .await
        .expect("Failed to run migrations");

    let tera = Tera::new("templates/*.html").expect("Failed to load templates");

    println!("Server running at http://127.0.0.1:8080");

    // Start server
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(db_pool.clone()))
            .app_data(web::Data::new(tera.clone()))
            .route("/", web::get().to(home_page))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
