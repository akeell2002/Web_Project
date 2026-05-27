use actix_web::{web, HttpResponse, Responder};
use actix_session::Session;
use sqlx::PgPool;
use tera::{Context, Tera};
use crate::models::medical_record::CreateMedicalRecordDto;
use crate::db::medical_records;

// GET: Show the form to add a medical record for a specific patient
pub async fn add_record_form(
    path: web::Path<i32>, // This extracts the patient_id from the URL
    tmpl: web::Data<Tera>,
    session: Session,
) -> impl Responder {
    let patient_id = path.into_inner();
    
    // Security check: Only let Doctors and Admins add records! (RBAC Requirement)
    let role: Option<String> = session.get("role").unwrap_or(None);
    if role.as_deref() != Some("doctor") && role.as_deref() != Some("admin") {
        return HttpResponse::Forbidden().body("Access Denied: Only Doctors can add medical records.");
    }

    // Get the logged-in doctor's ID from the cookie
    let doctor_id: i32 = session.get("user_id").unwrap().unwrap_or(0);

    let mut ctx = Context::new();
    ctx.insert("patient_id", &patient_id);
    ctx.insert("doctor_id", &doctor_id);

    // We will create this HTML file in the next step!
    let rendered = tmpl.render("medical_records/add.html", &ctx).unwrap();
    HttpResponse::Ok().content_type("text/html").body(rendered)
}

// POST: Process the submitted form data and save it to the database
pub async fn add_record(
    path: web::Path<i32>,
    form: web::Form<CreateMedicalRecordDto>,
    pool: web::Data<PgPool>,
) -> impl Responder {
    let patient_id = path.into_inner();
    let record_data = form.into_inner();

    // Call the database function we wrote earlier
    match medical_records::create_medical_record(&pool, record_data).await {
        Ok(_) => {
            // Success! Redirect the doctor back to the patient's main profile
            HttpResponse::SeeOther()
                .insert_header(("Location", format!("/patients/{}", patient_id)))
                .finish()
        }
        Err(e) => HttpResponse::InternalServerError().body(format!("Failed to save record: {}", e)),
    }
}