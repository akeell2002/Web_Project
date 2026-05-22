mod db;
mod models;

use actix_web::{web, App, HttpServer, HttpResponse, Responder};
use tera::Tera;

// Test if works
async fn health_check() -> impl Responder {
    HttpResponse::Ok().body("Patient Management System is running! lkakakakak")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize logging
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    
    println!("Starting Patient Management System...");
    
    // Create database connection pool
    let db_pool = db::create_db_pool().await.expect("Failed to create database pool");
    
    // Run migrations and create tables
    sqlx::migrate!().run(&db_pool).await.expect("Failed to run migrations");
    
    // Initialize Tera templates
    let tera = Tera::new("src/templates/**/*.html").expect("Failed to load templates");
    
    println!("Server running at http://127.0.0.1:8080");
    
    // Start server
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(db_pool.clone()))
            .app_data(web::Data::new(tera.clone()))
            .route("/health", web::get().to(health_check))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}