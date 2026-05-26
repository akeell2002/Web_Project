use actix_web::{web, HttpResponse, Responder};
use actix_session::Session;
use tera::{Tera, Context};
use sqlx::PgPool; 
use crate::models::patient::PatientForm;
use crate::db::patients as db_patients;

// GET /patients - Render list of all patients
pub async fn list_patients(
    tera: web::Data<Tera>, 
    pool: web::Data<PgPool>,
    session: Session
) -> impl Responder {
    if session.get::<i32>("user_id").unwrap_or(None).is_none() {
        return HttpResponse::SeeOther().append_header(("Location", "/login")).finish();
    }

    let mut ctx = Context::new();
    match db_patients::get_all_patients(&pool).await {
        Ok(patients) => {
            ctx.insert("patients", &patients);
            match tera.render("patients/list.html", &ctx) {
                Ok(html) => HttpResponse::Ok().content_type("text/html").body(html),
                Err(_) => HttpResponse::InternalServerError().body("Template Error"),
            }
        },
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

// GET /patients/add - Show creation form
pub async fn show_add_patient(tera: web::Data<Tera>, session: Session) -> impl Responder {
    if session.get::<i32>("user_id").unwrap_or(None).is_none() {
        return HttpResponse::SeeOther().append_header(("Location", "/login")).finish();
    }

    let ctx = Context::new();
    match tera.render("patients/add.html", &ctx) {
        Ok(html) => HttpResponse::Ok().content_type("text/html").body(html),
        Err(_) => HttpResponse::InternalServerError().body("Template Error"),
    }
}

// POST /patients/add - Capture form inputs and save them
pub async fn add_patient(
    pool: web::Data<PgPool>,
    form: web::Form<PatientForm>,
    session: Session
) -> impl Responder {
    let user_id = match session.get::<i32>("user_id").unwrap_or(None) {
        Some(id) => id,
        None => return HttpResponse::SeeOther().append_header(("Location", "/login")).finish(),
    };

    // Updated validation checking first and last names
    if form.first_name.trim().is_empty() || form.last_name.trim().is_empty() {
        return HttpResponse::BadRequest().body("Patient first and last names cannot be blank.");
    }

    match db_patients::create_patient(&pool, &form, user_id).await {
        Ok(_) => HttpResponse::SeeOther().append_header(("Location", "/patients")).finish(),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}