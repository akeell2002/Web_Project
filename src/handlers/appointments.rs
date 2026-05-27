use actix_web::{web, HttpResponse, Responder};
use actix_session::Session;
use sqlx::PgPool;
use tera::{Context, Tera};

use crate::db;
use crate::models::appointment::CreateAppointmentDto;

// 1. Render the booking page
pub async fn list_appointments(
    pool: web::Data<PgPool>,
    tmpl: web::Data<Tera>,
    session: Session,
) -> impl Responder {
    if session.get::<String>("username").unwrap_or(None).is_none() {
        return HttpResponse::SeeOther().append_header(("Location", "/login")).finish();
    }

    let appointments = db::appointments::get_all_appointments(&pool).await.unwrap_or_default();
    let patients = db::patients::get_all_patients(&pool).await.unwrap_or_default();
    let doctors = db::users::get_all_users(&pool).await.unwrap_or_default(); 

    let mut ctx = Context::new();
    ctx.insert("appointments", &appointments);
    ctx.insert("patients", &patients);
    ctx.insert("doctors", &doctors);
    ctx.insert("username", &session.get::<String>("username").unwrap().unwrap());
    ctx.insert("role", &session.get::<String>("role").unwrap().unwrap_or_default());

    let rendered = tmpl.render("appointments/list.html", &ctx).unwrap();
    HttpResponse::Ok().content_type("text/html").body(rendered)
}

// 2. Process form submission
pub async fn book_appointment(
    pool: web::Data<PgPool>,
    form: web::Form<CreateAppointmentDto>,
    session: Session,
) -> impl Responder {
    if session.get::<String>("username").unwrap_or(None).is_none() {
        return HttpResponse::SeeOther().append_header(("Location", "/login")).finish();
    }

    match db::appointments::create_appointment(&pool, form.into_inner()).await {
        Ok(_) => HttpResponse::SeeOther().append_header(("Location", "/appointments")).finish(),
        Err(err) => HttpResponse::InternalServerError().body(format!("Failed to schedule appointment: {}", err)),
    }
}
