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

// GET /patients/{id} - View a single patient profile
pub async fn view_patient(
    tera: web::Data<Tera>, 
    pool: web::Data<PgPool>,
    session: Session,
    path: web::Path<i32> // This extracts the {id} from the URL
) -> impl Responder {
    // 1. Security Check: Are they logged in?
    if session.get::<i32>("user_id").unwrap_or(None).is_none() {
        return HttpResponse::SeeOther().append_header(("Location", "/login")).finish();
    }

    let patient_id = path.into_inner();
    let mut ctx = Context::new();

    // 2. Fetch the specific patient
    match db_patients::get_patient_by_id(&pool, patient_id).await {
        Ok(patient) => {
            ctx.insert("patient", &patient);
            match tera.render("patients/profile.html", &ctx) {
                Ok(html) => HttpResponse::Ok().content_type("text/html").body(html),
                Err(e) => HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
            }
        },
        Err(sqlx::Error::RowNotFound) => {
            HttpResponse::NotFound().body("Patient not found in the database.")
        },
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
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
            match tera.render("patients/edit.html", &ctx) {
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