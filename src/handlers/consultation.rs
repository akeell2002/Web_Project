use actix_web::{web, HttpResponse, Responder};
use actix_session::Session;
use sqlx::PgPool;
use tera::{Context, Tera};
use uuid::Uuid;

use crate::models::consultation::EncounterForm;
use crate::db::consultation::finalize_consultation_and_bill;

// This handles the POST request when the doctor submits the form
pub async fn submit_consultation(
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>, // Extracts the appointment_id from the URL
    form: web::Form<EncounterForm>, // Deserializes the HTML form data
) -> impl Responder {
    let appointment_id = path.into_inner();
    let encounter_data = form.into_inner();

    // Call the database transaction from Step 2
    match finalize_consultation_and_bill(&pool, appointment_id, encounter_data).await {
        Ok(_) => {
            // If successful, redirect the doctor back to their queue
            HttpResponse::SeeOther()
                .insert_header(("Location", "/staff/doctor/queue"))
                .finish()
        }
        Err(e) => {
            // Log the error to the console for debugging
            eprintln!("Transaction Failed: {}", e);
            HttpResponse::InternalServerError().body("Failed to finalize consultation and billing.")
        }
    }
}

// NEW: This handles the GET request to display the blank HTML form to the doctor
pub async fn show_consultation_form(
    session: Session,
    tmpl: web::Data<Tera>,
    path: web::Path<Uuid>,
) -> impl Responder {
    // 1. Verify the user is a doctor
    match session.get::<String>("role") {
        Ok(Some(role)) if role == "doctor" => {},
        _ => return HttpResponse::SeeOther().append_header(("Location", "/staff/login")).finish(),
    };

    let appointment_id = path.into_inner();
    
    // 2. Pass the ID and blank default values to the HTML template
    let mut ctx = Context::new();
    ctx.insert("appointment_id", &appointment_id.to_string());
    
    // Tera requires all variables referenced in the HTML to exist in the context!
    // We pass empty strings so it successfully renders a blank form.
    ctx.insert("symptoms", "");
    ctx.insert("diagnosis", "");
    ctx.insert("treatment_notes", "");
    ctx.insert("medicine_name", "");
    ctx.insert("dosage", "");
    ctx.insert("frequency", "");
    ctx.insert("duration", "");
    ctx.insert("instructions", "");

    // 3. Render the consultation HTML page
    match tmpl.render("staff/doctor/consultation.html", &ctx) {
        Ok(html) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
        Err(e) => {
            eprintln!("Template Compilation Error: {}", e);
            HttpResponse::InternalServerError().body("Failed to load consultation form.")
        }
    }
}