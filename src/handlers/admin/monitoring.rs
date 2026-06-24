use actix_web::{web, HttpResponse, Responder};
use actix_session::Session;
use tera::{Tera, Context};
use sqlx::PgPool;

use crate::db::users::get_access_logs;
use crate::db::analytics::get_clinic_analytics;

use super::admin_only;

pub async fn security_monitoring_page(
    pool:    web::Data<PgPool>,
    session: Session,
    tmpl:    web::Data<Tera>,
) -> impl Responder {
    if let Err(response) = admin_only(&session) {
        return response;
    }

    let access_logs = match get_access_logs(&pool, 100).await {
        Ok(logs)     => logs,
        Err(err_msg) => return HttpResponse::InternalServerError().body(format!("Failed to load access logs: {}", err_msg)),
    };

    let email      = session.get::<String>("email").unwrap_or_default().unwrap_or_default();
    let display_name = crate::handlers::get_display_name(&session);

    let mut ctx = Context::new();
    ctx.insert("specific_role", "admin");
    ctx.insert("email",         &email);
    ctx.insert("display_name", &display_name);
    ctx.insert("access_logs",   &access_logs);
    ctx.insert("log_count",     &access_logs.len());

    match tmpl.render("admin/security.html", &ctx) {
        Ok(html) => HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .append_header(("Cache-Control", "no-store, no-cache, must-revalidate, max-age=0"))
            .body(html),
        Err(err) => HttpResponse::InternalServerError().body(format!("Template compilation error: {}", err)),
    }
}

pub async fn analytics_page(
    pool:    web::Data<PgPool>,
    session: Session,
    tmpl:    web::Data<Tera>,
) -> impl Responder {
    if let Err(response) = admin_only(&session) {
        return response;
    }

    let analytics = match get_clinic_analytics(&pool).await {
        Ok(a)  => a,
        Err(e) => return HttpResponse::InternalServerError().body(format!("Analytics query failed: {}", e)),
    };

    let email      = session.get::<String>("email").unwrap_or_default().unwrap_or_default();
    let display_name = crate::handlers::get_display_name(&session);

    let mut ctx = Context::new();
    ctx.insert("specific_role",       "admin");
    ctx.insert("email",               &email);
    ctx.insert("display_name", &display_name);
    ctx.insert("total_patients",      &analytics.total_patients);
    ctx.insert("total_appointments",  &analytics.total_appointments);
    ctx.insert("appts_today",         &analytics.appts_today);
    ctx.insert("appts_this_month",    &analytics.appts_this_month);
    ctx.insert("appts_completed",     &analytics.appts_completed);
    ctx.insert("appts_cancelled",     &analytics.appts_cancelled);
    ctx.insert("total_revenue",       &analytics.total_revenue);
    ctx.insert("revenue_this_month",  &analytics.revenue_this_month);
    ctx.insert("outstanding_bills",   &analytics.outstanding_bills);
    ctx.insert("total_prescriptions", &analytics.total_prescriptions);
    ctx.insert("total_doctors",       &analytics.total_doctors);
    ctx.insert("total_nurses",        &analytics.total_nurses);
    ctx.insert("total_receptionists", &analytics.total_receptionists);

    match tmpl.render("admin/analytics.html", &ctx) {
        Ok(html) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
        Err(e)   => HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
    }
}
