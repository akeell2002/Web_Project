use actix_web::{web, HttpResponse, Responder};
use actix_session::Session;
use tera::{Context, Tera};
use sqlx::PgPool;

use crate::db::staff::get_staff_dashboard_counts;

pub async fn patient_dashboard(
    session: Session,
    pool:    web::Data<PgPool>,
    tera:    web::Data<Tera>,
) -> impl Responder {
    if let Ok(Some(role)) = session.get::<String>("role") {
        if role == "patient" {
            let email      = session.get::<String>("email").unwrap_or_default().unwrap_or_default();
            let patient_id = session.get::<uuid::Uuid>("user_id").unwrap_or_default().unwrap_or_default();
            let staff_name = email.split('@').next().unwrap_or("Patient").to_string();

            let appointments = match crate::db::appointments::get_patient_appointments(&pool, patient_id).await {
                Ok(list) => list,
                Err(e) => {
                    eprintln!("Dashboard tracking query failure: {}", e);
                    Vec::new()
                }
            };

            let upcoming:   Vec<_> = appointments.iter().filter(|a|  a["is_upcoming"].as_bool().unwrap_or(false)).collect();
            let historical: Vec<_> = appointments.iter().filter(|a| !a["is_upcoming"].as_bool().unwrap_or(false)).collect();

            let mut ctx = Context::new();
            ctx.insert("email",                 &email);
            ctx.insert("specific_role",         "patient");
            ctx.insert("staff_name",            &staff_name);
            ctx.insert("upcoming_appointments", &upcoming);
            ctx.insert("historical_appointments", &historical);

            return match tera.render("shared/dashboard.html", &ctx) {
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
            let email        = session.get::<String>("email").unwrap_or_default().unwrap_or_default();
            let display_name = email.split('@').next().unwrap_or("Staff Member").to_string();

            let mut ctx = Context::new();
            ctx.insert("email",         &email);
            ctx.insert("staff_name",    &display_name);
            ctx.insert("specific_role", &role);

            return match tera.render("shared/dashboard.html", &ctx) {
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
            let email        = session.get::<String>("email").unwrap_or_default().unwrap_or_default();
            let display_name = email.split('@').next().unwrap_or("Admin").to_string();

            let counts = match get_staff_dashboard_counts(&pool).await {
                Ok(c)    => c,
                Err(err) => return HttpResponse::InternalServerError().body(format!("Failed to load staff counts: {}", err)),
            };

            let mut ctx = Context::new();
            ctx.insert("email",         &email);
            ctx.insert("specific_role", "admin");
            ctx.insert("staff_name",    &display_name);
            ctx.insert("staff_counts",  &counts);

            return match tera.render("shared/dashboard.html", &ctx) {
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
