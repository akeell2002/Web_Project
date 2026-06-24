use actix_web::{web, HttpResponse, Responder};
use actix_session::Session;
use tera::{Tera, Context};
use sqlx::PgPool;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct SupportTicketForm {
    pub submitter_name:      String,
    pub submitter_email:     String,
    pub issue_description:   String,
}

#[derive(Deserialize)]
pub struct ReplyForm {
    pub ticket_id:   Uuid,
    pub reply_notes: String,
}

#[derive(Deserialize)]
pub struct SupportQuery {
    pub sent: Option<String>,
}

#[derive(Deserialize)]
pub struct SupportDashQuery {
    pub replied: Option<String>,
}

/// GET /support — public contact form
pub async fn support_form_page(
    tmpl:  web::Data<Tera>,
    query: web::Query<SupportQuery>,
) -> impl Responder {
    let mut ctx = Context::new();
    let sent = query.sent.as_deref().unwrap_or("") == "1";
    ctx.insert("sent", &sent);
    match tmpl.render("support/form.html", &ctx) {
        Ok(html) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
        Err(e)   => HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
    }
}

/// POST /support/submit — anonymous ticket submission
pub async fn submit_support_ticket(
    pool: web::Data<PgPool>,
    form: web::Form<SupportTicketForm>,
) -> impl Responder {
    match crate::db::support::submit_ticket(
        &pool, &form.submitter_name, &form.submitter_email, &form.issue_description,
    )
    .await
    {
        Ok(_)  => HttpResponse::SeeOther().append_header(("Location", "/support?sent=1")).finish(),
        Err(e) => HttpResponse::InternalServerError().body(format!("Failed to submit ticket: {}", e)),
    }
}

/// GET /admin/support — admin support ticket dashboard
pub async fn support_dashboard(
    pool:    web::Data<PgPool>,
    session: Session,
    tmpl:    web::Data<Tera>,
    query:   web::Query<SupportDashQuery>,
) -> impl Responder {
    match session.get::<String>("role") {
        Ok(Some(role)) if role == "admin" => {}
        Ok(Some(_)) => return HttpResponse::Forbidden().body("Access Denied"),
        _ => return HttpResponse::SeeOther().append_header(("Location", "/staff/login")).finish(),
    }

    let email        = session.get::<String>("email").unwrap_or_default().unwrap_or_default();
    let display_name = crate::handlers::get_display_name(&session);
    let current_role = session.get::<String>("role").unwrap_or_default().unwrap_or_default();

    let tickets = match crate::db::support::get_all_tickets(&pool).await {
        Ok(t)  => t,
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };

    let replied = query.replied.as_deref().unwrap_or("") == "1";

    let mut ctx = Context::new();
    ctx.insert("tickets",       &tickets);
    ctx.insert("email",         &email);
    ctx.insert("display_name", &display_name);
    ctx.insert("specific_role", &current_role);
    ctx.insert("replied",       &replied);

    match tmpl.render("admin/support_dashboard.html", &ctx) {
        Ok(html) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
        Err(e)   => HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
    }
}

/// POST /admin/support/reply — admin sends reply
pub async fn submit_reply(
    pool:    web::Data<PgPool>,
    session: Session,
    form:    web::Form<ReplyForm>,
) -> impl Responder {
    match session.get::<String>("role") {
        Ok(Some(role)) if role == "admin" => {}
        Ok(Some(_)) => return HttpResponse::Forbidden().body("Access Denied"),
        _ => return HttpResponse::SeeOther().append_header(("Location", "/staff/login")).finish(),
    }

    match crate::db::support::reply_to_ticket(&pool, form.ticket_id, &form.reply_notes).await {
        Ok(_)  => HttpResponse::SeeOther().append_header(("Location", "/admin/support?replied=1")).finish(),
        Err(e) => HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    }
}
