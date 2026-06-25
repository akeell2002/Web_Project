use actix_web::{web, HttpResponse, Responder};
use actix_session::Session;
use sqlx::PgPool;
use uuid::Uuid;
use tera::{Tera, Context};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct QueueFilterParams {
    pub view: Option<String>,
}

/// GET - doctor's daily clinical queue
pub async fn doctor_daily_queue_page(
    pool:    web::Data<PgPool>,
    session: Session,
    tmpl:    web::Data<Tera>,
    query:   web::Query<QueueFilterParams>,
) -> impl Responder {
    match session.get::<String>("role") {
        Ok(Some(role)) if role == "doctor" => {}
        _ => return HttpResponse::SeeOther().append_header(("Location", "/staff/login")).finish(),
    }

    let doctor_id = match session.get::<Uuid>("user_id") {
        Ok(Some(id)) => id,
        _ => return HttpResponse::SeeOther().append_header(("Location", "/staff/login")).finish(),
    };

    let filter_mode = match query.view.as_deref() {
        Some("all") => "all",
        _           => "today",
    };

    let today      = chrono::Local::now().date_naive();
    let email      = session.get::<String>("email").unwrap_or_default().unwrap_or_default();
    let display_name = crate::handlers::get_display_name(&session);

    let appointments = match crate::db::appointments::get_doctor_daily_appointments(&pool, doctor_id, filter_mode).await {
        Ok(data) => data,
        Err(e)   => return HttpResponse::InternalServerError().body(format!("Clinical query failure: {}", e)),
    };

    let mut ctx = Context::new();
    ctx.insert("specific_role",   "doctor");
    ctx.insert("email",           &email);
    ctx.insert("display_name", &display_name);
    ctx.insert("formatted_date",  &today.format("%A, %B %d, %Y").to_string());
    ctx.insert("queue",           &appointments);
    ctx.insert("queue_count",     &appointments.len());
    ctx.insert("current_view",    filter_mode);

    match tmpl.render("staff/doctor/doctor_queue.html", &ctx) {
        Ok(html) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
        Err(e)   => HttpResponse::InternalServerError().body(format!("Rendering error: {}", e)),
    }
}

/// GET - consultation form for a specific appointment
pub async fn show_consultation_form(
    pool:    web::Data<PgPool>,
    session: Session,
    tmpl:    web::Data<Tera>,
    path:    web::Path<Uuid>,
) -> impl Responder {
    match session.get::<String>("role") {
        Ok(Some(role)) if role == "doctor" => {}
        _ => return HttpResponse::SeeOther().append_header(("Location", "/staff/login")).finish(),
    }

    let appointment_id = path.into_inner();
    let email          = session.get::<String>("email").unwrap_or_default().unwrap_or_default();
    let display_name   = crate::handlers::get_display_name(&session);

    let patient_info = match crate::db::appointments::get_consultation_patient_info(&pool, appointment_id).await {
        Ok(Some(info)) => info,
        Ok(None)       => serde_json::Value::Null,
        Err(e)         => {
            eprintln!("Failed to load patient info for consultation: {}", e);
            serde_json::Value::Null
        }
    };

    let mut ctx = Context::new();
    ctx.insert("specific_role",    "doctor");
    ctx.insert("email",            &email);
    ctx.insert("display_name",     &display_name);
    ctx.insert("appointment_id",   &appointment_id.to_string());
    ctx.insert("patient",          &patient_info);
    ctx.insert("symptoms",         "");
    ctx.insert("diagnosis",        "");
    ctx.insert("treatment_notes",  "");
    ctx.insert("medicine_name",    "");
    ctx.insert("dosage",           "");
    ctx.insert("frequency",        "");
    ctx.insert("duration",         "");
    ctx.insert("instructions",     "");

    match tmpl.render("staff/doctor/consultation.html", &ctx) {
        Ok(html) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
        Err(e)   => {
            eprintln!("Template error (consultation): {}", e);
            HttpResponse::InternalServerError().body("Failed to load consultation form.")
        }
    }
}

/// POST - submit consultation, create medical record, bill, close appointment
pub async fn submit_consultation(
    pool:    web::Data<PgPool>,
    session: Session,
    path:    web::Path<Uuid>,
    form:    web::Form<crate::models::appointment::EncounterForm>,
) -> impl Responder {
    // Only a doctor may finalize a consultation (and decide on admission).
    match session.get::<String>("role") {
        Ok(Some(role)) if role == "doctor" => {}
        _ => return HttpResponse::Forbidden().body("Access Denied: Doctor access required."),
    }

    let appointment_id = path.into_inner();
    let admitted = form.action.as_deref() == Some("admit");

    match crate::db::appointments::finalize_consultation_and_bill(&pool, appointment_id, form.into_inner()).await {
        Ok(_)  => {
            let location = if admitted {
                "/staff/doctor/queue?success=admitted"
            } else {
                "/staff/doctor/queue?success=consultation_saved"
            };
            HttpResponse::SeeOther().insert_header(("Location", location)).finish()
        }
        Err(e) => {
            eprintln!("Consultation transaction failed: {}", e);
            HttpResponse::InternalServerError().body("Failed to finalize consultation and billing.")
        }
    }
}

#[derive(Deserialize)]
pub struct PrescribeForm {
    pub medicine_name: String,
    pub dosage:        String,
    pub frequency:     String,
    pub duration:      String,
    pub instructions:  Option<String>,
}

/// GET - prescribe medication page (per-patient cards)
pub async fn prescribe_medication_page(
    pool:    web::Data<PgPool>,
    session: Session,
    tmpl:    web::Data<Tera>,
) -> impl Responder {
    match session.get::<String>("role") {
        Ok(Some(role)) if role == "doctor" => {}
        _ => return HttpResponse::SeeOther().append_header(("Location", "/staff/login")).finish(),
    }

    let doctor_id  = match session.get::<Uuid>("user_id") {
        Ok(Some(id)) => id,
        _ => return HttpResponse::SeeOther().append_header(("Location", "/staff/login")).finish(),
    };

    let email      = session.get::<String>("email").unwrap_or_default().unwrap_or_default();
    let display_name = crate::handlers::get_display_name(&session);

    let appointments = match crate::db::appointments::get_doctor_prescribable_appointments(&pool, doctor_id).await {
        Ok(list) => list,
        Err(e)   => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };

    let mut ctx = Context::new();
    ctx.insert("specific_role", "doctor");
    ctx.insert("email",         &email);
    ctx.insert("display_name", &display_name);
    ctx.insert("appointments",  &appointments);
    ctx.insert("success",       &false);

    match tmpl.render("staff/doctor/prescribe.html", &ctx) {
        Ok(html) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
        Err(e)   => HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
    }
}

/// POST - issue a prescription for a specific appointment
pub async fn submit_prescription(
    pool:    web::Data<PgPool>,
    session: Session,
    path:    web::Path<Uuid>,
    form:    web::Form<PrescribeForm>,
) -> impl Responder {
    let doctor_id = match session.get::<Uuid>("user_id") {
        Ok(Some(id)) => id,
        _ => return HttpResponse::SeeOther().append_header(("Location", "/staff/login")).finish(),
    };

    let appointment_id = path.into_inner();

    match crate::db::appointments::insert_prescription(
        &pool, appointment_id, doctor_id,
        &form.medicine_name, &form.dosage, &form.frequency, &form.duration,
        form.instructions.as_deref(),
    )
    .await
    {
        Ok(_) => {
            eprintln!(
                "\n[MOCK EMAIL] Prescription: {} ({} x {} for {}) -- appt {}\n",
                form.medicine_name, form.dosage, form.frequency, form.duration, appointment_id
            );
            HttpResponse::SeeOther().append_header(("Location", "/staff/doctor/prescribe?success=1")).finish()
        }
        Err(e) => HttpResponse::InternalServerError().body(format!("Failed to save prescription: {}", e)),
    }
}