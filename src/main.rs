pub mod db;
pub mod models;

use actix_web::{web, App, HttpServer, HttpResponse, Responder};
use tera::{Tera, Context}; // Added Context to pass variables to HTML


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
    // Initialize logging (Make sure env_logger is in your Cargo.toml if you uncomment this!)
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    
    println!("Starting Patient Management System...");
    
    let db_pool = db::create_db_pool().await.expect("Failed to create database pool");
    sqlx::migrate!().run(&db_pool).await.expect("Failed to run migrations");
    
    // Fixed the Tera directory path (removed "src/")
    let tera = Tera::new("templates/**/*.html").expect("Failed to load templates");
    
    println!("Server running at http://127.0.0.1:8080");
    
    // Start server
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(db_pool.clone())) // Commented out for now
            .app_data(web::Data::new(tera.clone()))
            .route("/", web::get().to(home_page))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}