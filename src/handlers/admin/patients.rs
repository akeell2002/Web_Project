use actix_web::{web, HttpResponse, Responder};
use actix_session::Session;
use tera::{Tera, Context};
use sqlx::PgPool;

// Handler for the patient directory page
pub async fn patient_directory_page(
    pool:    web::Data<PgPool>,
    session: Session,
    tmpl:    web::Data<Tera>,
) -> impl Responder {
    if let Err(response) = super::super::staff_only(&session) {
        return response;
    }

    let patients = match crate::db::patients::get_patient_directory(&pool).await {
        Ok(rows)     => rows,
        Err(err_msg) => return HttpResponse::InternalServerError().body(format!("Failed to load patients: {}", err_msg)),
    };

    let current_role = session.get::<String>("role").unwrap_or_default().unwrap_or_default();
    let email        = session.get::<String>("email").unwrap_or_default().unwrap_or_default();
    let display_name = crate::handlers::get_display_name(&session);

    let mut ctx = Context::new();
    ctx.insert("patients",      &patients);
    ctx.insert("specific_role", &current_role);
    ctx.insert("email",         &email);
    ctx.insert("display_name", &display_name);

    match tmpl.render("staff/patient_directory.html", &ctx) {
        Ok(html) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
        Err(e)   => HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
    }
}
