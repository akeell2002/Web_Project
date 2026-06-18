// src/handlers/patients.rs
use actix_web::{web, HttpResponse, Responder};
use actix_session::Session;
use tera::{Tera, Context};
use sqlx::PgPool;
use serde::Deserialize;
use crate::models::patient::CreatePatientProfile;

#[derive(Deserialize)]
pub struct AddPatientForm {
    pub first_name: String,
    pub last_name: String,
    pub date_of_birth: chrono::NaiveDate,
    pub gender: String,
    pub phone_number: Option<String>,
    pub email: String,
    pub raw_password: String,
}

fn staff_only(session: &Session) -> Result<(), HttpResponse> {
    match session.get::<String>("role") {
        Ok(Some(role)) if (role == "doctor" || role == "nurse" || role == "receptionist" || role == "admin") => Ok(()),
        Ok(Some(_)) => Err(HttpResponse::Forbidden().body("Access Denied: Staff access required.")),
        _ => Err(HttpResponse::SeeOther().append_header(("Location", "/staff/login")).finish()),
    }
}

pub async fn show_add_patient_page(
    session: Session, 
    tmpl: web::Data<Tera>
) -> impl Responder {
    if let Err(response) = staff_only(&session) {
        return response;
    }

    let current_role = session.get::<String>("role").unwrap_or_default().unwrap_or_default();
    let email = session.get::<String>("email").unwrap_or_default().unwrap_or_default();
    let staff_name = email.split('@').next().unwrap_or("Staff").to_string();

    let mut ctx = Context::new();
    ctx.insert("specific_role", &current_role);
    ctx.insert("email", &email);
    ctx.insert("staff_name", &staff_name);

    // UPDATED: Points to the secure staff directory template path
    match tmpl.render("staff/add.html", &ctx) {
        Ok(html) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
        Err(e) => HttpResponse::InternalServerError().body(format!("Form layout load error: {}", e)),
    }
}

pub async fn process_add_patient(
    pool: web::Data<PgPool>,
    session: Session,
    form: web::Form<AddPatientForm>,
) -> impl Responder {
    if let Err(response) = staff_only(&session) {
        return response;
    }

    let profile = CreatePatientProfile {
        first_name: form.first_name.clone(),
        last_name: form.last_name.clone(),
        date_of_birth: form.date_of_birth,
        gender: Some(form.gender.clone()),
        phone_number: form.phone_number.clone(),
        emergency_contact_name: None,
        emergency_contact_phone: None,
    };

    match crate::db::patients::register_patient(
        pool.get_ref(),
        &form.email,
        &form.raw_password,
        profile,
    ).await {
        Ok(_) => {
            HttpResponse::SeeOther()
                .append_header(("Location", "/staff/patients?success=patient_registered"))
                .finish()
        }
        Err(e) => HttpResponse::BadRequest().body(format!("Database transactional update failed: {}", e)),
    }
}