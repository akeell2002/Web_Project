mod db;
mod models;
mod utils;
mod handlers;

use actix_web::{App, HttpServer, HttpResponse, Responder, web};
use actix_session::{SessionMiddleware, storage::CookieSessionStore};
use actix_web::cookie::Key;
use tera::{Context, Tera};

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

    let tera = Tera::new("templates/**/*.html").expect("Failed to load templates");
    //to encrypt session cookies
    let secret_key = Key::generate();

    println!("Server running at http://127.0.0.1:8080");

    // Start server
HttpServer::new(move || {
    App::new()
        // First add session middleware so it's available in all routes [cite: 426]
        .wrap(SessionMiddleware::builder(CookieSessionStore::default(), secret_key.clone()).build())
        
        // Then all the shared data - database pool and template engine [cite: 326, 422, 425]
        .app_data(web::Data::new(db_pool.clone()))
        .app_data(web::Data::new(tera.clone()))
        
        // Serves your Instagram-style CSS files securely [cite: 597, 598]
        .service(actix_files::Files::new("/static", "./static").show_files_listing()) 
        
        // Index route (Renders your homepage template) [cite: 325, 362]
        .route("/", web::get().to(home_page))
        
        // Auth routes [cite: 436, 439]
        .route("/login", web::get().to(handlers::auth::show_login))
        .route("/login", web::post().to(handlers::auth::login))
        .route("/register", web::get().to(handlers::auth::show_register))
        .route("/register", web::post().to(handlers::auth::register))
        
        // Logged in user routes [cite: 442, 446]
        .route("/dashboard", web::get().to(handlers::auth::dashboard))
        .route("/logout", web::get().to(handlers::auth::logout))

        // Patient management routes
        .route("/patients", web::get().to(handlers::patients::list_patients))
        .route("/patients/add", web::get().to(handlers::patients::show_add_patient))
        .route("/patients/add", web::post().to(handlers::patients::add_patient))
})
.bind(("127.0.0.1", 8080))?
.run()
.await
}
