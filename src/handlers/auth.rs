use actix_web::{web, HttpResponse, Responder, HttpRequest};
use actix_session::Session;
use tera::{Context, Tera};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::user::{LoginForm, PatientRegisterForm, UserRole};
use crate::db::users::{authenticate_user, find_user_by_email};
use crate::db::staff::get_staff_dashboard_counts;
use crate::db::patients::register_patient;
use crate::db::security::log_access_event;

// Staff login page rendering
pub async fn staff_login(tera: web::Data<Tera>) -> impl Responder {
    let ctx = Context::new();
    match tera.render("staff/login.html", &ctx) {
        Ok(html) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
        Err(e) => HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
    }
}

// Patient login page rendering
pub async fn patient_login(tera: web::Data<Tera>) -> impl Responder {
    let ctx = Context::new();
    match tera.render("patient/login.html", &ctx) {
        Ok(html) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
        Err(e) => HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
    }
}

// Centralized Processing for ALL login submissions (Both Staff & Patients)
pub async fn login(
    pool: web::Data<PgPool>,
    tera: web::Data<Tera>,
    req: HttpRequest,
    form: web::Form<LoginForm>,
    session: Session,
) -> impl Responder {
    // 1. Authenticate credentials against our email schema
    match authenticate_user(&pool, &form.email, &form.password).await {
        Ok(Some(user)) => {
            // 2. Initialize session parameters securely
            let _ = session.insert("user_id", user.id);
            let _ = session.insert("email", &user.email);
            
            // Serialize our type-safe enum directly into the cookie string
            let role_str = match user.role {
                UserRole::Admin => "admin",
                UserRole::Doctor => "doctor",
                UserRole::Nurse => "nurse",
                UserRole::Receptionist => "receptionist",
                UserRole::Patient => "patient",
            };
            let _ = session.insert("role", role_str);

            if let Err(err) = log_access_event(
                pool.get_ref(),
                Some(user.id),
                Some(&user.email),
                "login_success",
                Some(user.id),
                &user.email,
                role_str,
                &format!("{} logged in successfully.", user.email),
            )
            .await
            {
                eprintln!("Security log write failed for login_success: {}", err);
            }

            // 3. Smart routing: Send users to their dedicated dashboard structures!
            let redirect_target = match user.role {
                UserRole::Admin => "/admin/dashboard",
                UserRole::Doctor | UserRole::Nurse | UserRole::Receptionist => "/staff/dashboard",
                UserRole::Patient => "/patient/dashboard",
            };

            HttpResponse::SeeOther()
                .append_header(("Location", redirect_target))
                .finish()
        }
        Ok(None) => {
            // Authentication failed — choose the correct login template based on request path
            let mut ctx = Context::new();
            ctx.insert("error_message", &"Invalid email or password.");
            ctx.insert("email", &form.email);

            // Default to staff login; if the incoming path is under /patient, render patient login
            let path = req.path();
            let template_name = if path.starts_with("/patient") {
                "patient/login.html"
            } else {
                "staff/login.html"
            };

            return match tera.render(template_name, &ctx) {
                Ok(html) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
                Err(e) => HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
            };
        }
        Err(e) => HttpResponse::InternalServerError().body(format!("System Authentication Error: {}", e)),
    }
}

// Render patient registration view
pub async fn show_register(tera: web::Data<Tera>) -> impl Responder {
    let ctx = Context::new();
    match tera.render("patient/register.html", &ctx) {
        Ok(html) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
        Err(e) => HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
    }
}

// Process self-registration for patient accounts
pub async fn register(
    pool: web::Data<PgPool>,
    form: web::Form<PatientRegisterForm>,
    session: Session,
) -> impl Responder {
    if form.password != form.confirm_password {
        return HttpResponse::BadRequest().body("Passwords do not match!");
    }

    // Build the sub-profile block
    let profile = crate::models::patient::CreatePatientProfile {
        first_name: form.first_name.clone(),
        last_name: form.last_name.clone(),
        date_of_birth: chrono::NaiveDate::parse_from_str(&form.date_of_birth, "%d/%m/%Y").unwrap_or_else(|_| chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap()),
        gender: form.gender.clone(),
        phone_number: form.phone_number.clone(),
        emergency_contact_name: form.emergency_contact_name.clone(),
        emergency_contact_phone: form.emergency_contact_phone.clone(),
    };

    // Invoke our SQL transaction runner
    match register_patient(&pool, &form.email, &form.password, profile).await {
        Ok(user) => {
            // Log the user into their session automatically upon sign-up
            let _ = session.insert("user_id", user.id);
            let _ = session.insert("email", &user.email);
            let _ = session.insert("role", "patient");

            HttpResponse::SeeOther()
                .append_header(("Location", "/patient/dashboard"))
                .finish()
        }
        Err(e) => HttpResponse::InternalServerError().body(format!("Registration transactional error: {}", e)),
    }
}

// Explicit Dashboard Gatekeepers
pub async fn patient_dashboard(
    session: actix_session::Session, 
    pool: web::Data<sqlx::PgPool>, 
    tera: web::Data<tera::Tera>
) -> impl Responder {
    if let Ok(Some(role)) = session.get::<String>("role") {
        if role == "patient" {
            let email = session.get::<String>("email").unwrap_or_default().unwrap_or_default();
            let patient_id = session.get::<uuid::Uuid>("user_id").unwrap_or_default().unwrap_or_default();
            
            let mut ctx = tera::Context::new();
            ctx.insert("email", &email);

            // Fetch the dynamic active and past tracking records from the database layer
            let appointments = match crate::db::appointments::get_patient_appointments(&pool, patient_id).await {
                Ok(list) => list,
                Err(e) => {
                    eprintln!("Dashboard tracking query failure: {}", e);
                    Vec::new()
                }
            };

            // Filter into two clear timeline slices based on our helper identifier flag
            let upcoming: Vec<_> = appointments.iter().filter(|a| a["is_upcoming"].as_bool().unwrap_or(false)).collect();
            let historical: Vec<_> = appointments.iter().filter(|a| !a["is_upcoming"].as_bool().unwrap_or(false)).collect();

            ctx.insert("upcoming_appointments", &upcoming);
            ctx.insert("historical_appointments", &historical);

            return match tera.render("patient/dashboard.html", &ctx) {
                Ok(html) => HttpResponse::Ok()
                    .content_type("text/html")
                    .append_header(("Cache-Control", "no-store, no-cache, must-revalidate, max-age=0"))
                    .body(html),
                Err(e) => HttpResponse::InternalServerError().body(format!("Dashboard template error: {}", e)),
            };
        }
    }
    HttpResponse::SeeOther().append_header(("Location", "/patient/login")).finish()
}

pub async fn staff_dashboard(session: Session, tera: web::Data<Tera>) -> impl Responder {
    if let Ok(Some(role)) = session.get::<String>("role") {
        if role == "doctor" || role == "nurse" || role == "receptionist" {
            let email = session.get::<String>("email").unwrap_or_default().unwrap_or_default();
            let mut ctx = Context::new();
            ctx.insert("email", &email);

            // Split the email prefix as a quick substitute for their name string display
            let display_name = email.split('@').next().unwrap_or("Staff Member");
            ctx.insert("staff_name", display_name); 
            ctx.insert("specific_role", &role);
            
            return match tera.render("staff/dashboard.html", &ctx) {
                Ok(html) => HttpResponse::Ok()
                    .content_type("text/html")
                    .append_header(("Cache-Control", "no-store, no-cache, must-revalidate, max-age=0"))
                    .append_header(("Pragma", "no-cache"))
                    .body(html),
                Err(e) => HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
            };
        }
    }
    HttpResponse::SeeOther().append_header(("Location", "/staff/login")).finish()
}

pub async fn admin_dashboard(session: Session, pool: web::Data<PgPool>, tera: web::Data<Tera>) -> impl Responder {
    if let Ok(Some(role)) = session.get::<String>("role") {
        if role == "admin" {
            let email = session.get::<String>("email").unwrap_or_default().unwrap_or_default();
            let mut ctx = Context::new();
            ctx.insert("email", &email);

            let counts = match get_staff_dashboard_counts(&pool).await {
                Ok(counts) => counts,
                Err(err) => {
                    return HttpResponse::InternalServerError().body(format!("Failed to load staff counts: {}", err));
                }
            };

            ctx.insert("staff_counts", &counts);
            
            return match tera.render("admin/dashboard.html", &ctx) {
                Ok(html) => HttpResponse::Ok()
                    .content_type("text/html")
                    .append_header(("Cache-Control", "no-store, no-cache, must-revalidate, max-age=0"))
                    .append_header(("Pragma", "no-cache"))
                    .body(html),
                Err(e) => HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
            };
        }
    }
    HttpResponse::SeeOther().append_header(("Location", "/staff/login")).finish()
}

// Centralized Session Cleanup
pub async fn logout(pool: web::Data<PgPool>, session: Session) -> impl Responder {
    let current_user_id = session.get::<Uuid>("user_id").unwrap_or_default();
    let current_email = session.get::<String>("email").unwrap_or_default().unwrap_or_default();
    let current_role = session.get::<String>("role").unwrap_or_default().unwrap_or_default();

    if !current_email.is_empty() {
        let role_label = match current_role.as_str() {
            "admin" => "admin",
            "doctor" => "doctor",
            "nurse" => "nurse",
            "receptionist" => "receptionist",
            "patient" => "patient",
            _ => "unknown",
        };

        if let Err(err) = log_access_event(
            pool.get_ref(),
            current_user_id,
            Some(&current_email),
            "logout_success",
            current_user_id,
            &current_email,
            role_label,
            &format!("{} logged out successfully.", current_email),
        )
        .await
        {
            eprintln!("Security log write failed for logout_success: {}", err);
        }
    }

    session.clear();
    
    // Clean redirect based on who is leaving
    if current_role == "patient" {
        HttpResponse::SeeOther().append_header(("Location", "/patient/login")).finish()
    } else {
        HttpResponse::SeeOther().append_header(("Location", "/staff/login")).finish()
    }
}