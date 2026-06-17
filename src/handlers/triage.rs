// src/handlers/triage.rs
use actix_web::{web, HttpResponse, Responder};
use actix_session::Session;
use sqlx::PgPool;
use uuid::Uuid;
use tera::{Tera, Context};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct SubmitVitalsForm {
    pub blood_pressure: String,
    pub temperature: String,
    pub weight_kg: String,
    pub height_cm: String,
}

/// Renders the Nurse Triage Dashboard
pub async fn nurse_triage_page(
    pool: web::Data<PgPool>,
    session: Session,
    tmpl: web::Data<Tera>,
) -> impl Responder {
    // Only allow nurses (and admins)
    match session.get::<String>("role") {
        Ok(Some(role)) if role == "nurse" || role == "admin" => {},
        _ => return HttpResponse::SeeOther().append_header(("Location", "/staff/nurse/triage?success=true")).finish(),
    };

    let queue = crate::db::triage::get_triage_queue(&pool).await.unwrap_or_default();
    
    let mut ctx = Context::new();
    ctx.insert("queue", &queue);
    ctx.insert("date", &chrono::Local::now().format("%A, %B %d, %Y").to_string());

    ctx.insert("specific_role", &session.get::<String>("role").unwrap_or_default().unwrap_or_default());

    // UPDATED: Now targeting the staff/nurse/ folder structure layout
    match tmpl.render("staff/nurse/nurse_triage.html", &ctx) {
        Ok(html) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
        Err(e) => HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
    }
}

/// POST route to save vitals
pub async fn submit_triage_vitals(
    pool: web::Data<PgPool>,
    session: Session,
    path: web::Path<Uuid>, 
    form: web::Form<SubmitVitalsForm>,
) -> impl Responder {
    
    let nurse_id = match session.get::<Uuid>("user_id") {
        Ok(Some(id)) => id,
        _ => return HttpResponse::SeeOther().append_header(("Location", "/staff/login")).finish(),
    };

    let appointment_id = path.into_inner();

    match crate::db::triage::record_patient_vitals(
        &pool, 
        appointment_id, 
        nurse_id, 
        form.blood_pressure.clone(), 
        form.temperature.clone(), 
        form.weight_kg.clone(), 
        form.height_cm.clone()
    ).await {
        Ok(_) => {
            // UPDATED: Dynamic redirection match with main.rs path mapping setup
            HttpResponse::SeeOther()
                .append_header(("Location", "/staff/nurse/triage?success=vitals_saved"))
                .finish()
        },
        Err(e) => HttpResponse::BadRequest().body(e),
    }
}