// src/handlers/patients.rs
use actix_web::{web, HttpResponse, Responder};
use actix_session::Session;
use tera::{Tera, Context};
use sqlx::PgPool;
use serde::Deserialize;
use uuid::Uuid;
use chrono::NaiveDate;
use crate::models::patient::CreatePatientProfile;

#[derive(Deserialize)]
pub struct AddPatientForm {
    pub first_name:              String,
    pub last_name:               String,
    pub date_of_birth:           NaiveDate,
    pub gender:                  String,
    pub phone_number:            Option<String>,
    pub emergency_contact_name:  Option<String>,
    pub emergency_contact_phone: Option<String>,
    pub email:                   String,
    pub raw_password:            String,
}

#[derive(Deserialize)]
pub struct EditPatientForm {
    pub first_name:              String,
    pub last_name:               String,
    pub date_of_birth:           NaiveDate,
    pub gender:                  Option<String>,
    pub phone_number:            Option<String>,
    pub emergency_contact_name:  Option<String>,
    pub emergency_contact_phone: Option<String>,
}


pub async fn show_add_patient_page(
    session: Session, 
    tmpl: web::Data<Tera>
) -> impl Responder {
    if let Err(response) = super::staff_only(&session) {
        return response;
    }

    let current_role = session.get::<String>("role").unwrap_or_default().unwrap_or_default();
    let email = session.get::<String>("email").unwrap_or_default().unwrap_or_default();
    let display_name = crate::handlers::get_display_name(&session);

    let mut ctx = Context::new();
    ctx.insert("specific_role", &current_role);
    ctx.insert("email", &email);
    ctx.insert("display_name", &display_name);

    // UPDATED: Points to the secure staff directory template path
    match tmpl.render("staff/add.html", &ctx) {
        Ok(html) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
        Err(e) => HttpResponse::InternalServerError().body(format!("Form layout load error: {}", e)),
    }
}

pub async fn patient_detail_page(
    pool: web::Data<PgPool>,
    session: Session,
    tmpl: web::Data<Tera>,
    path: web::Path<Uuid>,
) -> impl Responder {
    if let Err(response) = super::staff_only(&session) {
        return response;
    }

    let patient_id = path.into_inner();
    let current_role = session.get::<String>("role").unwrap_or_default().unwrap_or_default();
    let email = session.get::<String>("email").unwrap_or_default().unwrap_or_default();
    let display_name = crate::handlers::get_display_name(&session);

    match crate::db::patients::get_patient_detail(pool.get_ref(), patient_id).await {
        Ok(Some(patient)) => {
            let mut ctx = Context::new();
            ctx.insert("specific_role", &current_role);
            ctx.insert("email", &email);
            ctx.insert("display_name", &display_name);
            ctx.insert("patient", &patient);

            match tmpl.render("staff/patient_detail.html", &ctx) {
                Ok(html) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
                Err(e) => HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Patient not found."),
        Err(e) => HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    }
}

pub async fn patient_report_page(
    pool: web::Data<PgPool>,
    session: Session,
    tmpl: web::Data<Tera>,
    path: web::Path<Uuid>,
) -> impl Responder {
    if let Err(response) = super::staff_only(&session) {
        return response;
    }

    let patient_id = path.into_inner();

    match crate::db::patients::get_patient_detail(pool.get_ref(), patient_id).await {
        Ok(Some(patient)) => {
            let report_date = chrono::Local::now().format("%d %b %Y").to_string();
            let mut ctx = Context::new();
            ctx.insert("patient", &patient);
            ctx.insert("report_date", &report_date);

            match tmpl.render("staff/patient_report.html", &ctx) {
                Ok(html) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
                Err(e) => HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Patient not found."),
        Err(e) => HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    }
}

pub async fn process_add_patient(
    pool: web::Data<PgPool>,
    session: Session,
    form: web::Form<AddPatientForm>,
) -> impl Responder {
    if let Err(response) = super::staff_only(&session) {
        return response;
    }

    let profile = CreatePatientProfile {
        first_name:              form.first_name.clone(),
        last_name:               form.last_name.clone(),
        date_of_birth:           form.date_of_birth,
        gender:                  Some(form.gender.clone()),
        phone_number:            form.phone_number.clone(),
        emergency_contact_name:  form.emergency_contact_name.clone(),
        emergency_contact_phone: form.emergency_contact_phone.clone(),
    };

    match crate::db::patients::register_patient(
        pool.get_ref(),
        &form.email,
        &form.raw_password,
        profile,
    ).await {
        Ok(_) => {
            HttpResponse::SeeOther()
                .append_header(("Location", "/staff/patients?success=patient_created"))
                .finish()
        }
        Err(e) => HttpResponse::BadRequest().body(format!("Database transactional update failed: {}", e)),
    }
}

/// GET /staff/patients/{id}/edit - show edit form pre-filled with current data
pub async fn show_edit_patient_page(
    pool:    web::Data<PgPool>,
    session: Session,
    tmpl:    web::Data<Tera>,
    path:    web::Path<Uuid>,
) -> impl Responder {
    if let Err(r) = super::staff_only(&session) { return r; }

    let patient_id   = path.into_inner();
    let current_role = session.get::<String>("role").unwrap_or_default().unwrap_or_default();
    let email        = session.get::<String>("email").unwrap_or_default().unwrap_or_default();
    let display_name = crate::handlers::get_display_name(&session);

    match crate::db::patients::get_patient_detail(pool.get_ref(), patient_id).await {
        Ok(Some(patient)) => {
            let mut ctx = Context::new();
            ctx.insert("specific_role", &current_role);
            ctx.insert("email",         &email);
            ctx.insert("display_name", &display_name);
            ctx.insert("patient",       &patient);
            match tmpl.render("patient/edit.html", &ctx) {
                Ok(html) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
                Err(e)   => HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Patient not found."),
        Err(e)   => HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    }
}

/// POST /staff/patients/{id}/edit - save updated demographics
pub async fn process_edit_patient(
    pool:    web::Data<PgPool>,
    session: Session,
    path:    web::Path<Uuid>,
    form:    web::Form<EditPatientForm>,
) -> impl Responder {
    if let Err(r) = super::staff_only(&session) { return r; }

    let patient_id = path.into_inner();
    let email = session.get::<String>("email").unwrap_or_default().unwrap_or_default();

    match crate::db::patients::update_patient_profile(
        pool.get_ref(),
        &email,
        patient_id,
        &form.first_name,
        &form.last_name,
        form.date_of_birth,
        form.gender.clone(),
        form.phone_number.clone(),
        form.emergency_contact_name.clone(),
        form.emergency_contact_phone.clone(),
    ).await {
        Ok(_)  => HttpResponse::SeeOther()
            .append_header(("Location", format!("/staff/patients/{}", patient_id)))
            .finish(),
        Err(e) => HttpResponse::InternalServerError().body(format!("Update failed: {}", e)),
    }
}

/// POST /staff/patients/{id}/delete - remove patient account
pub async fn process_delete_patient(
    pool:    web::Data<PgPool>,
    session: Session,
    path:    web::Path<Uuid>,
) -> impl Responder {
    // Only admin can delete patients
    match session.get::<String>("role") {
        Ok(Some(role)) if role == "admin" => {}
        _ => return HttpResponse::Forbidden().body("Access Denied: Admin access required."),
    }

    let patient_id = path.into_inner();
    let admin_email = session.get::<String>("email").unwrap_or_default().unwrap_or_default();
    let patient_email = match sqlx::query_scalar!(
        "SELECT email FROM users WHERE id = $1 AND role = 'patient'::user_role",
        patient_id
    )
    .fetch_optional(pool.get_ref())
    .await {
        Ok(Some(email)) => email,
        Ok(None) => return HttpResponse::NotFound().body("Target patient account not found."),
        Err(e) => return HttpResponse::InternalServerError().body(format!("Database registry error: {}", e)),
    };

    match crate::db::patients::delete_patient(pool.get_ref(), patient_id, &admin_email, &patient_email).await {
        Ok(_)  => HttpResponse::SeeOther()
            .append_header(("Location", "/staff/patients?success=patient_deleted"))
            .finish(),
        Err(e) => HttpResponse::InternalServerError().body(format!("Delete failed: {}", e)),
    }
}