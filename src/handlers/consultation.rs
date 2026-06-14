use actix_web::{web, HttpResponse, Responder};
use actix_session::Session;
use sqlx::PgPool;
use tera::{Tera, Context};
use uuid::Uuid;

/// Renders the Doctor's Daily Queue
pub async fn doctor_queue_page(
    pool: web::Data<PgPool>,
    session: Session,
    tmpl: web::Data<Tera>,
) -> impl Responder {
    
    // Only allow doctors and admins
    let role = session.get::<String>("role").unwrap_or_default().unwrap_or_default();
    if role != "doctor" && role != "admin" {
        return HttpResponse::SeeOther().append_header(("Location", "/staff/dashboard")).finish();
    }

    // Get the ID of the logged-in doctor
    let doctor_id = match session.get::<Uuid>("user_id") {
        Ok(Some(id)) => id,
        _ => return HttpResponse::SeeOther().append_header(("Location", "/staff/login")).finish(),
    };

    let queue = crate::db::consultation::get_doctor_queue(&pool, doctor_id).await.unwrap_or_default();
    
    let mut ctx = Context::new();
    ctx.insert("specific_role", &role); // Pass role for the Navbar!
    ctx.insert("queue", &queue);
    ctx.insert("date", &chrono::Local::now().format("%A, %B %d, %Y").to_string());

    match tmpl.render("staff/doctor_queue.html", &ctx) {
        Ok(html) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
        Err(e) => HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
    }
}

/// Renders the individual patient consultation room workspace
pub async fn consultation_room_page(
    pool: web::Data<sqlx::PgPool>,
    session: actix_session::Session,
    tmpl: web::Data<tera::Tera>,
    path: web::Path<uuid::Uuid>,
) -> impl actix_web::Responder {
    
    let role = session.get::<String>("role").unwrap_or_default().unwrap_or_default();
    if role != "doctor" && role != "admin" {
        return actix_web::HttpResponse::SeeOther().append_header(("Location", "/staff/dashboard")).finish();
    }

    let appointment_id = path.into_inner();

    match crate::db::consultation::get_consultation_details(&pool, appointment_id).await {
        Ok(details) => {
            let mut ctx = tera::Context::new();
            ctx.insert("specific_role", &role);
            ctx.insert("patient", &details);
            ctx.insert("date", &chrono::Local::now().format("%A, %B %d, %Y").to_string());

            match tmpl.render("staff/consultation.html", &ctx) {
                Ok(html) => actix_web::HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
                Err(e) => actix_web::HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
            }
        }
        Err(e) => actix_web::HttpResponse::NotFound().body(e),
    }
}