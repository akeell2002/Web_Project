use actix_web::{web, HttpResponse, Responder};
use actix_session::Session;
use sqlx::PgPool;
use tera::{Context, Tera};
use uuid::Uuid;
use serde::Deserialize;

use crate::db;

// ─── BED MANAGEMENT PAGE ─────────────────────────────────────────────────────

pub async fn bed_management_page(
    pool: web::Data<PgPool>,
    session: Session,
    tera: web::Data<Tera>,
) -> impl Responder {
    if let Err(r) = super::staff_only(&session) {
        return r;
    }

    let role  = session.get::<String>("role").unwrap_or(None).unwrap_or_default();
    let email = session.get::<String>("email").unwrap_or(None).unwrap_or_default();
    let display_name = crate::handlers::get_display_name(&session);

    // Fetch all data in parallel (sequentially for simplicity)
    let beds = db::beds::get_bed_overview(&pool).await.unwrap_or_default();
    let bed_stats = db::beds::get_bed_stats(&pool).await
        .unwrap_or_else(|_| serde_json::json!({"total_beds":0,"available":0,"occupied":0,"maintenance":0}));

    let patients = db::beds::get_patient_census(&pool).await.unwrap_or_default();
    let patient_stats = db::beds::get_patient_stats(&pool).await
        .unwrap_or_else(|_| serde_json::json!({"total_patients":0,"emergency":0,"vitals_taken":0,"discharged_today":0}));

    let transfers = db::beds::get_transfer_requests(&pool).await.unwrap_or_default();

    let mut ctx = Context::new();
    ctx.insert("specific_role", &role);
    ctx.insert("display_name", &display_name);
    ctx.insert("email",         &email);
    ctx.insert("beds",          &beds);
    ctx.insert("bed_stats",     &bed_stats);
    ctx.insert("patients",      &patients);
    ctx.insert("patient_stats", &patient_stats);
    ctx.insert("transfers",     &transfers);

    match tera.render("staff/bed_management.html", &ctx) {
        Ok(html) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
        Err(e)   => {
            eprintln!("Template error (bed_management): {}", e);
            HttpResponse::InternalServerError().body("Template error")
        }
    }
}

// ─── APPROVE TRANSFER ────────────────────────────────────────────────────────

pub async fn approve_transfer_handler(
    pool: web::Data<PgPool>,
    session: Session,
    path: web::Path<Uuid>,
) -> impl Responder {
    // Doctor-only gate
    match session.get::<String>("role") {
        Ok(Some(r)) if r == "doctor" || r == "admin" => {}
        _ => return HttpResponse::Forbidden().body("Only doctors can approve transfers"),
    }

    let doctor_id = match session.get::<Uuid>("user_id").unwrap_or(None) {
        Some(id) => id,
        None => return HttpResponse::Unauthorized().body("Not authenticated"),
    };

    let transfer_id = path.into_inner();

    match db::beds::approve_transfer(&pool, transfer_id, doctor_id).await {
        Ok(_) => HttpResponse::SeeOther()
            .append_header(("Location", "/staff/beds"))
            .finish(),
        Err(e) => {
            eprintln!("approve_transfer error: {}", e);
            HttpResponse::InternalServerError().body(e)
        }
    }
}

// ─── REJECT TRANSFER ─────────────────────────────────────────────────────────

pub async fn reject_transfer_handler(
    pool: web::Data<PgPool>,
    session: Session,
    path: web::Path<Uuid>,
) -> impl Responder {
    match session.get::<String>("role") {
        Ok(Some(r)) if r == "doctor" || r == "admin" => {}
        _ => return HttpResponse::Forbidden().body("Only doctors can reject transfers"),
    }

    let doctor_id = match session.get::<Uuid>("user_id").unwrap_or(None) {
        Some(id) => id,
        None => return HttpResponse::Unauthorized().body("Not authenticated"),
    };

    let transfer_id = path.into_inner();

    match db::beds::reject_transfer(&pool, transfer_id, doctor_id).await {
        Ok(_) => HttpResponse::SeeOther()
            .append_header(("Location", "/staff/beds"))
            .finish(),
        Err(e) => {
            eprintln!("reject_transfer error: {}", e);
            HttpResponse::InternalServerError().body(e)
        }
    }
}

// ─── DISCHARGE ADMITTED PATIENT (doctor only) ────────────────────────────────

pub async fn discharge_patient_handler(
    pool: web::Data<PgPool>,
    session: Session,
    path: web::Path<Uuid>,
) -> impl Responder {
    // Doctor-only gate: nurses and receptionists cannot discharge.
    match session.get::<String>("role") {
        Ok(Some(r)) if r == "doctor" || r == "admin" => {}
        _ => return HttpResponse::Forbidden().body("Only doctors can discharge patients"),
    }

    let appointment_id = path.into_inner();

    match db::beds::discharge_patient(&pool, appointment_id).await {
        Ok(_) => HttpResponse::SeeOther()
            .append_header(("Location", "/staff/beds?success=discharged"))
            .finish(),
        Err(e) => {
            eprintln!("discharge_patient error: {}", e);
            HttpResponse::SeeOther()
                .append_header(("Location", "/staff/beds?error=discharge_failed"))
                .finish()
        }
    }
}

// ─── REQUEST TRANSFER (POST form) ────────────────────────────────────────────

#[derive(Deserialize)]
pub struct TransferForm {
    pub patient_id:   String,
    pub from_room_id: Option<String>,
    pub to_room_id:   String,
    pub reason:       Option<String>,
}

pub async fn request_transfer_handler(
    pool: web::Data<PgPool>,
    session: Session,
    form: web::Form<TransferForm>,
) -> impl Responder {
    if let Err(r) = super::staff_only(&session) {
        return r;
    }

    let requester_id = match session.get::<Uuid>("user_id").unwrap_or(None) {
        Some(id) => id,
        None => return HttpResponse::Unauthorized().body("Not authenticated"),
    };

    let patient_id = match Uuid::parse_str(&form.patient_id) {
        Ok(u) => u,
        Err(_) => return HttpResponse::BadRequest().body("Invalid patient ID"),
    };
    let to_room_id = match Uuid::parse_str(&form.to_room_id) {
        Ok(u) => u,
        Err(_) => return HttpResponse::BadRequest().body("Invalid room ID"),
    };
    let from_room_id = form
        .from_room_id
        .as_deref()
        .and_then(|s| Uuid::parse_str(s).ok());

    match db::beds::create_transfer_request(
        &pool,
        patient_id,
        from_room_id,
        to_room_id,
        requester_id,
        form.reason.clone(),
    )
    .await
    {
        Ok(_) => HttpResponse::SeeOther()
            .append_header(("Location", "/staff/beds"))
            .finish(),
        Err(e) => {
            eprintln!("create_transfer_request error: {}", e);
            HttpResponse::InternalServerError().body(e)
        }
    }
}
