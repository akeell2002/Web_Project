use actix_web::{web, HttpResponse, Responder};
use actix_session::Session;
use tera::{Context, Tera};
use sqlx::PgPool;

use crate::models::user::PatientRegisterForm;
use crate::db::patients::register_patient;

pub async fn show_register(tera: web::Data<Tera>) -> impl Responder {
    let ctx = Context::new();
    match tera.render("patient/register.html", &ctx) {
        Ok(html) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
        Err(e)   => HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
    }
}

pub async fn register(
    pool:    web::Data<PgPool>,
    form:    web::Form<PatientRegisterForm>,
    session: Session,
) -> impl Responder {
    if form.password != form.confirm_password {
        return HttpResponse::BadRequest().body("Passwords do not match!");
    }

    let profile = crate::models::patient::CreatePatientProfile {
        first_name:              form.first_name.clone(),
        last_name:               form.last_name.clone(),
        date_of_birth:           chrono::NaiveDate::parse_from_str(&form.date_of_birth, "%d/%m/%Y")
                                     .unwrap_or_else(|_| chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap()),
        gender:                  form.gender.clone(),
        phone_number:            form.phone_number.clone(),
        emergency_contact_name:  form.emergency_contact_name.clone(),
        emergency_contact_phone: form.emergency_contact_phone.clone(),
    };

    match register_patient(&pool, &form.email, &form.password, profile).await {
        Ok(_user) => {
            // Don't auto-login — send to login page with a success banner instead
            HttpResponse::SeeOther()
                .append_header(("Location", "/patient/login?registered=1"))
                .finish()
        }
        Err(e) => HttpResponse::InternalServerError().body(format!("Registration transactional error: {}", e)),
    }
}
