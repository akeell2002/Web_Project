use actix_web::{web, HttpResponse, Responder};
use actix_session::Session;
use tera::{Tera, Context};
use sqlx::PgPool; 
use crate::models::patient::PatientForm;
use crate::db::patients as db_patients;
use crate::db::medical_records;

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
            match tera.render("patient/list.html", &ctx) {
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
    match tera.render("patient/add.html", &ctx) {
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

// GET /patients/{id} - View a single patient profile
pub async fn view_patient(
    path: web::Path<i32>,
    pool: web::Data<sqlx::PgPool>,
    tmpl: web::Data<tera::Tera>,
    session: actix_session::Session,
) -> impl actix_web::Responder {
    // Basic auth check
    if session.get::<i32>("user_id").unwrap_or(None).is_none() {
        return actix_web::HttpResponse::SeeOther().insert_header(("Location", "/login")).finish();
    }

    let patient_id = path.into_inner();
    
    // 1. Get Patient Details
    let patient = crate::db::patients::get_patient_by_id(&pool, patient_id).await.unwrap();
    
    // 2. Get Appointments
    let appointments = crate::db::appointments::get_appointments_by_patient(&pool, patient_id).await.unwrap_or_default();
    
    // 3.Get Medical Records (This fixes your warning!)
    let records = medical_records::get_records_by_patient(&pool, patient_id).await.unwrap_or_default();

    let mut ctx = tera::Context::new();
    ctx.insert("patient", &patient);
    ctx.insert("appointments", &appointments);
    ctx.insert("medical_records", &records); // Pass records to the HTML

    let rendered = tmpl.render("patient/profile.html", &ctx).unwrap();
    actix_web::HttpResponse::Ok().content_type("text/html").body(rendered)
}

// GET /patients/{id}/edit - Show the pre-filled edit form
pub async fn show_edit_patient(
    tera: web::Data<Tera>, 
    pool: web::Data<PgPool>,
    session: Session,
    path: web::Path<i32>
) -> impl Responder {
    if session.get::<i32>("user_id").unwrap_or(None).is_none() {
        return HttpResponse::SeeOther().append_header(("Location", "/login")).finish();
    }

    let patient_id = path.into_inner();
    let mut ctx = Context::new();

    match db_patients::get_patient_by_id(&pool, patient_id).await {
        Ok(patient) => {
            ctx.insert("patient", &patient);
            match tera.render("patient/edit.html", &ctx) {
                Ok(html) => HttpResponse::Ok().content_type("text/html").body(html),
                Err(e) => HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
            }
        },
        Err(_) => HttpResponse::NotFound().body("Patient not found."),
    }
}

// POST /patients/{id}/edit - Save the updated data
pub async fn edit_patient(
    pool: web::Data<PgPool>,
    form: web::Form<PatientForm>,
    session: Session,
    path: web::Path<i32>
) -> impl Responder {
    if session.get::<i32>("user_id").unwrap_or(None).is_none() {
        return HttpResponse::SeeOther().append_header(("Location", "/login")).finish();
    }

    let patient_id = path.into_inner();

    if form.first_name.trim().is_empty() || form.last_name.trim().is_empty() {
        return HttpResponse::BadRequest().body("Names cannot be blank.");
    }

    match db_patients::update_patient(&pool, patient_id, &form).await {
        // Redirect back to their profile page after a successful save!
        Ok(_) => HttpResponse::SeeOther().append_header(("Location", format!("/patients/{}", patient_id))).finish(),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

// POST /patients/{id}/delete - Destroy the record
pub async fn delete_patient_handler(
    pool: web::Data<PgPool>,
    session: Session,
    path: web::Path<i32>
) -> impl Responder {
    // Security check
    if session.get::<i32>("user_id").unwrap_or(None).is_none() {
        return HttpResponse::SeeOther().append_header(("Location", "/login")).finish();
    }

    let patient_id = path.into_inner();

    match db_patients::delete_patient(&pool, patient_id).await {
        Ok(_) => HttpResponse::SeeOther().append_header(("Location", "/patients")).finish(),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}



