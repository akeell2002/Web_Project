pub mod db;
pub mod models;
pub mod utils;
pub mod handlers;

use actix_web::{App, HttpServer, HttpResponse, Responder, web};
use actix_session::{SessionMiddleware, storage::CookieSessionStore};
use actix_web::cookie::Key;
use tera::{Context, Tera};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use handlers::auth::ResetTokenStore;

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

    // Seed 100 fake patients for testing (skips if already seeded)
    if let Err(e) = crate::db::seed_test_data::seed_test_patients(&db_pool).await {
        eprintln!("Test data seeding warning: {}", e);
    }

    let tera = Tera::new("templates/**/*.html").expect("Failed to load templates");
    let reset_token_store: ResetTokenStore = Arc::new(Mutex::new(HashMap::new()));
    
    //to encrypt session cookies, use it after testing is done
    //let secret_key = Key::generate();

    // For development, we can use a static key
    let session_secret = std::env::var("SESSION_SECRET")
        .unwrap_or_else(|_| "a_very_long_and_secure_static_64_byte_secret_for_dev_testing_purposes!!".to_string());
    let secret_key = Key::from(session_secret.as_bytes());

    println!("Server running at http://127.0.0.1:8080");

    // Start server
    HttpServer::new(move || {
        App::new()
            // First add session middleware so it's available in all routes
            .wrap(SessionMiddleware::builder(CookieSessionStore::default(), secret_key.clone()).build())
            
            // Then all the shared data, database pool and template engine
            .app_data(web::Data::new(db_pool.clone()))
            .app_data(web::Data::new(tera.clone()))
            .app_data(web::Data::new(reset_token_store.clone()))
            
            // Static files
            .service(actix_files::Files::new("/static", "./static").show_files_listing()) 
            
            // Index route
            .route("/", web::get().to(home_page))

            // Admin interface routes
            .route("/admin/dashboard", web::get().to(handlers::auth::admin_dashboard))
            .route("/admin/security", web::get().to(handlers::admin::security_monitoring_page))
            .route("/admin/analytics", web::get().to(handlers::admin::analytics_page))
            .route("/admin/staff/onboard", web::get().to(handlers::admin::onboard_staff_page))
            .route("/admin/staff/onboard", web::post().to(handlers::admin::onboard_staff_submit))
            .route("/admin/staff", web::get().to(handlers::admin::staff_directory_page))

            // === PUBLIC INDEX ROUTES ===
            .route("/support", web::get().to(handlers::admin::support_form_page))
            .route("/support/submit", web::post().to(handlers::admin::submit_support_ticket))

            // === STAFF INTERFACE ROUTES ===
            .route("/staff/login", web::get().to(handlers::auth::staff_login))
            .route("/staff/login", web::post().to(handlers::auth::login))
            .route("/staff/dashboard", web::get().to(handlers::auth::staff_dashboard))
            .route("/staff/patients", web::get().to(handlers::admin::patient_directory_page))
            .route("/staff/patients/add", web::get().to(handlers::patients::show_add_patient_page))
            .route("/staff/patients/add", web::post().to(handlers::patients::process_add_patient))
            .route("/staff/patients/{id}", web::get().to(handlers::patients::patient_detail_page))
            .route("/staff/patients/{id}/report", web::get().to(handlers::patients::patient_report_page))

            // --- Doctor Routes ---
            .route("/staff/doctor/queue", web::get().to(handlers::appointments::doctor_daily_queue_page))
            .route("/staff/doctor/patients", web::get().to(handlers::appointments::doctor_daily_queue_page))
            .route("/staff/doctor/prescribe", web::get().to(handlers::appointments::prescribe_medication_page))
            .route("/staff/doctor/prescribe/{id}", web::post().to(handlers::appointments::submit_prescription))
            .route("/staff/doctor/consultation/{id}", web::get().to(handlers::appointments::show_consultation_form))
            .route("/staff/doctor/consultation/{id}", web::post().to(handlers::appointments::submit_consultation))

            // --- Shared Bed Management ---
            .route("/staff/beds", web::get().to(handlers::beds::bed_management_page))
            .route("/staff/beds/transfer/request", web::post().to(handlers::beds::request_transfer_handler))
            .route("/staff/beds/transfer/{id}/approve", web::post().to(handlers::beds::approve_transfer_handler))
            .route("/staff/beds/transfer/{id}/reject", web::post().to(handlers::beds::reject_transfer_handler))

            // --- Nurse Routes ---
            .route("/staff/nurse/triage", web::get().to(handlers::appointments::nurse_triage_page))
            .route("/staff/nurse/queue/triage/{id}", web::post().to(handlers::appointments::submit_triage_vitals))
            .route("/staff/nurse/medications", web::get().to(handlers::appointments::medication_administration_page))
            .route("/staff/nurse/medications/{id}/administer", web::post().to(handlers::appointments::submit_medication_administration))

            // --- Receptionist Routes ---
            .route("/staff/receptionist/reception", web::get().to(handlers::appointments::reception_desk_page))
            .route("/staff/receptionist/queue/check_in/{id}", web::post().to(handlers::appointments::process_check_in))
            .route("/staff/receptionist/billing", web::get().to(handlers::billing::show_billing_dashboard))
            .route("/staff/receptionist/billing/checkout", web::post().to(handlers::billing::checkout_bill_submit))
            .route("/admin/support", web::get().to(handlers::admin::support_dashboard))
            .route("/admin/support/reply", web::post().to(handlers::admin::submit_reply))

            // Patient interface routes
            .route("/patient/login", web::get().to(handlers::auth::patient_login))
            .route("/patient/login", web::post().to(handlers::auth::login)) // Binds patient login form submission here
            .route("/patient/register", web::get().to(handlers::auth::show_register))
            .route("/patient/register", web::post().to(handlers::auth::register))
            .route("/patient/dashboard", web::get().to(handlers::auth::patient_dashboard))

            // Patient profile
            .route("/patient/profile", web::get().to(handlers::auth::patient_profile_page))

            // Staff profile
            .route("/staff/profile", web::get().to(handlers::auth::staff_profile_page))

            // Patient appointment scheduling endpoints
            .route("/patient/appointments/book", web::get().to(handlers::appointments::show_booking_form))
            .route("/patient/appointments/create", web::post().to(handlers::appointments::submit_appointment))
            .route("/patient/appointments/{id}/cancel", web::post().to(handlers::appointments::cancel_appointment))

            // Password reset routes
            .route("/forgot-password", web::get().to(handlers::auth::forgot_password_page))
            .route("/forgot-password", web::post().to(handlers::auth::submit_forgot_password))
            .route("/reset-password", web::get().to(handlers::auth::reset_password_page))
            .route("/reset-password", web::post().to(handlers::auth::submit_reset_password))

            // Logout route
            .route("/logout", web::get().to(handlers::auth::logout))
            
        })
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}