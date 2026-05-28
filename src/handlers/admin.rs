use actix_web::{web, HttpResponse, Responder};
use actix_session::Session;
use tera::{Tera, Context};
use sqlx::PgPool;
use crate::models::user::UserRole;
use crate::models::staff::{AddDoctorForm, CreateStaffProfile};
use crate::db::staff::register_staff;

/// GET request handler
pub async fn add_doctor_page(tmpl: web::Data<Tera>, session: Session) -> impl Responder {
    // Security Guard: Only let logged-in admins see this page
    if let Ok(Some(role)) = session.get::<String>("role") {
        if role != "admin" {
            return HttpResponse::Forbidden().body("Access Denied: Admin access required.");
        }
    } else {
        return HttpResponse::SeeOther().append_header(("Location", "/staff/login")).finish();
    }

    let ctx = Context::new();
    match tmpl.render("admin/add_doctor.html", &ctx) {
        Ok(html) => HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .append_header(("Cache-Control", "no-store, no-cache, must-revalidate, max-age=0"))
            .body(html),
        Err(e) => HttpResponse::InternalServerError().body(format!("Template compilation error: {}", e)),
    }
}

/// To process the form submission for adding a new doctor
pub async fn add_doctor_submit(
    pool: web::Data<PgPool>,
    session: Session,
    form: web::Form<AddDoctorForm>,
) -> impl Responder {
    // Security Guard: Double check authentication role criteria
    if let Ok(Some(role)) = session.get::<String>("role") {
        if role != "admin" {
            return HttpResponse::Forbidden().body("Access Denied.");
        }
    } else {
        return HttpResponse::SeeOther().append_header(("Location", "/staff/login")).finish();
    }

    // Wrap the profile pieces to match register_staff signature
    let staff_profile = CreateStaffProfile {
        first_name: form.first_name.clone(),
        last_name: form.last_name.clone(),
        phone_number: form.phone_number.clone(),
    };

    // Execute the transaction with the type-safe Doctor role
    match register_staff(&pool, &form.email, &form.password, UserRole::Doctor, staff_profile).await {
        Ok(_) => {
            // Success! Send them back to the admin dashboard
            HttpResponse::SeeOther()
                .append_header(("Location", "/admin/dashboard?success=doctor_added"))
                .finish()
        }
        Err(err_msg) => {
            HttpResponse::BadRequest().body(format!("Failed to register doctor: {}", err_msg))
        }
    }
}