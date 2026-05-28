mod db;
mod models;
mod utils;
mod handlers;

use actix_web::{App, HttpServer, HttpResponse, Responder, web};
use actix_session::{SessionMiddleware, storage::CookieSessionStore};
use actix_web::cookie::Key;
use tera::{Context, Tera};

// Index.html
async fn home_page(tera: web::Data<Tera>) -> impl Responder {
    let ctx = Context::new();

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

// Main function to start the server
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

    let tera = Tera::new("templates/**/*.html").expect("Failed to load templates");
    
    //to encrypt session cookies
    let secret_key = Key::generate();

    println!("Server running at http://127.0.0.1:8080");

    // Start server
    HttpServer::new(move || {
        App::new()
            // First add session middleware so it's available in all routes
            .wrap(SessionMiddleware::builder(CookieSessionStore::default(), secret_key.clone()).build())
            
            // Then all the shared data, database pool and template engine
            .app_data(web::Data::new(db_pool.clone()))
            .app_data(web::Data::new(tera.clone()))
            
            // Static files
            .service(actix_files::Files::new("/static", "./static").show_files_listing()) 
            
            // Index route
            .route("/", web::get().to(home_page))

            // Staff routes
            .route("/staff/login", web::get().to(handlers::auth::staff_login))

            // Patient routes
            .route("/patient/login", web::get().to(handlers::auth::patient_login))
            .route("/patient/login", web::post().to(handlers::auth::login))
            .route("/patient/register", web::get().to(handlers::auth::show_register))
            .route("/patient/register", web::post().to(handlers::auth::register))
            
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
    }