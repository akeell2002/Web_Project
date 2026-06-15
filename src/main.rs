pub mod db;
pub mod models;
pub mod utils;
pub mod handlers;

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

    // Seed the default staff accounts if they are not already present
    if let Err(e) = crate::db::users::seed_default_staff_users(&db_pool).await {
        eprintln!("System initialization warning: Resetting seed failed: {}", e);
    }

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

            // Admin interface routes
            .route("/admin/dashboard", web::get().to(handlers::auth::admin_dashboard))
            .route("/admin/security", web::get().to(handlers::admin::security_monitoring_page))
            .route("/admin/staff/onboard", web::get().to(handlers::admin::onboard_staff_page))
            .route("/admin/staff/onboard", web::post().to(handlers::admin::onboard_staff_submit))
            .route("/admin/staff", web::get().to(handlers::admin::staff_directory_page))

            // === PUBLIC INDEX ROUTES ===
            .route("/support", web::get().to(handlers::support::support_form_page))
            .route("/support/submit", web::post().to(handlers::support::submit_support_ticket))

            // === STAFF INTERFACE ROUTES ===
            .route("/staff/login", web::get().to(handlers::auth::staff_login))
            .route("/staff/login", web::post().to(handlers::auth::login))
            .route("/staff/dashboard", web::get().to(handlers::auth::staff_dashboard))
            .route("/staff/patients", web::get().to(handlers::admin::patient_directory_page))
            .route("/staff/patients/add", web::get().to(handlers::patients::show_add_patient_page))
            .route("/staff/patients/add", web::post().to(handlers::patients::process_add_patient))

            // --- Doctor Routes ---
            .route("/staff/doctor/queue", web::get().to(handlers::appointments::doctor_daily_queue_page))
            .route("/staff/doctor/patients", web::get().to(handlers::appointments::doctor_daily_queue_page))

            // --- Nurse Routes ---
            .route("/staff/nurse/triage", web::get().to(handlers::triage::nurse_triage_page))
            .route("/staff/nurse/queue/triage/{id}", web::post().to(handlers::triage::submit_triage_vitals))

            // --- Receptionist Routes ---
            .route("/staff/receptionist/reception", web::get().to(handlers::appointments::reception_desk_page))
            .route("/staff/receptionist/queue/check_in/{id}", web::post().to(handlers::appointments::process_check_in))
            .route("/staff/receptionist/support", web::get().to(handlers::receptionist::support_dashboard))
            .route("/staff/receptionist/support/reply", web::post().to(handlers::receptionist::submit_reply))

            // Patient interface routes
            .route("/patient/login", web::get().to(handlers::auth::patient_login))
            .route("/patient/login", web::post().to(handlers::auth::login)) // Binds patient login form submission here
            .route("/patient/register", web::get().to(handlers::auth::show_register))
            .route("/patient/register", web::post().to(handlers::auth::register))
            .route("/patient/dashboard", web::get().to(handlers::auth::patient_dashboard))

            // Patient appointment scheduling endpoints
            .route("/patient/appointments/book", web::get().to(handlers::appointments::show_booking_form))
            .route("/patient/appointments/create", web::post().to(handlers::appointments::submit_appointment))

            // Logout route
            .route("/logout", web::get().to(handlers::auth::logout))
            
        })
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}