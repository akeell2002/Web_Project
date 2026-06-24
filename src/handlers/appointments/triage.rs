use actix_web::{web, HttpResponse, Responder};
use actix_session::Session;
use sqlx::PgPool;
use uuid::Uuid;
use tera::{Tera, Context};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct SubmitVitalsForm {
    pub blood_pressure: String,
    pub temperature:    String,
    pub weight_kg:      String,
    pub height_cm:      String,
    pub priority_level: i32,
}

/// GET - nurse triage queue
pub async fn nurse_triage_page(
    pool:    web::Data<PgPool>,
    session: Session,
    tmpl:    web::Data<Tera>,
) -> impl Responder {
    match session.get::<String>("role") {
        Ok(Some(role)) if role == "nurse" || role == "admin" => {}
        _ => return HttpResponse::SeeOther().append_header(("Location", "/staff/login")).finish(),
    }

    let queue        = crate::db::appointments::get_triage_queue(&pool).await.unwrap_or_default();
    let current_role = session.get::<String>("role").unwrap_or_default().unwrap_or_default();
    let email        = session.get::<String>("email").unwrap_or_default().unwrap_or_default();
    let display_name = crate::handlers::get_display_name(&session);

    let mut ctx = Context::new();
    ctx.insert("specific_role", &current_role);
    ctx.insert("email",         &email);
    ctx.insert("display_name", &display_name);
    ctx.insert("queue",         &queue);
    ctx.insert("date",          &chrono::Local::now().format("%A, %B %d, %Y").to_string());

    match tmpl.render("staff/nurse/nurse_triage.html", &ctx) {
        Ok(html) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
        Err(e)   => HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
    }
}

/// POST - nurse submits triage vitals
pub async fn submit_triage_vitals(
    pool:    web::Data<PgPool>,
    session: Session,
    path:    web::Path<Uuid>,
    form:    web::Form<SubmitVitalsForm>,
) -> impl Responder {
    let nurse_id = match session.get::<Uuid>("user_id") {
        Ok(Some(id)) => id,
        _ => return HttpResponse::SeeOther().append_header(("Location", "/staff/login")).finish(),
    };

    let appointment_id = path.into_inner();

    match crate::db::appointments::record_patient_vitals(
        &pool,
        appointment_id,
        nurse_id,
        form.blood_pressure.clone(),
        form.temperature.clone(),
        form.weight_kg.clone(),
        form.height_cm.clone(),
        form.priority_level,
    )
    .await
    {
        Ok(_)  => HttpResponse::SeeOther().append_header(("Location", "/staff/nurse/triage?success=vitals_saved")).finish(),
        Err(e) => HttpResponse::BadRequest().body(e),
    }
}

/// GET - nurse medication administration page
pub async fn medication_administration_page(
    pool:    web::Data<PgPool>,
    session: Session,
    tmpl:    web::Data<Tera>,
) -> impl Responder {
    match session.get::<String>("role") {
        Ok(Some(role)) if role == "nurse" || role == "admin" => {}
        _ => return HttpResponse::SeeOther().append_header(("Location", "/staff/login")).finish(),
    }

    let email        = session.get::<String>("email").unwrap_or_default().unwrap_or_default();
    let display_name = crate::handlers::get_display_name(&session);
    let current_role = session.get::<String>("role").unwrap_or_default().unwrap_or_default();

    let prescriptions = match crate::db::appointments::get_active_prescriptions_for_nurse(&pool).await {
        Ok(list) => list,
        Err(e)   => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };

    let mut ctx = Context::new();
    ctx.insert("specific_role",  &current_role);
    ctx.insert("email",          &email);
    ctx.insert("display_name", &display_name);
    ctx.insert("prescriptions",  &prescriptions);

    match tmpl.render("staff/nurse/medications.html", &ctx) {
        Ok(html) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
        Err(e)   => HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
    }
}

#[derive(Deserialize)]
pub struct AdminLogForm {
    pub remarks: Option<String>,
}

/// POST - nurse logs medication administration
pub async fn submit_medication_administration(
    pool:    web::Data<PgPool>,
    session: Session,
    path:    web::Path<Uuid>,
    form:    web::Form<AdminLogForm>,
) -> impl Responder {
    let nurse_id = match session.get::<Uuid>("user_id") {
        Ok(Some(id)) => id,
        _ => return HttpResponse::SeeOther().append_header(("Location", "/staff/login")).finish(),
    };

    let prescription_id = path.into_inner();

    match crate::db::appointments::log_medication_administration(
        &pool, prescription_id, nurse_id, form.remarks.clone(),
    )
    .await
    {
        Ok(_)  => HttpResponse::SeeOther().append_header(("Location", "/staff/nurse/medications?success=logged")).finish(),
        Err(e) => HttpResponse::InternalServerError().body(format!("Failed: {}", e)),
    }
}
