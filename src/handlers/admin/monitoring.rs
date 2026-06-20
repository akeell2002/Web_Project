use actix_web::{web, HttpResponse, Responder};
use actix_session::Session;
use tera::{Tera, Context};
use sqlx::PgPool;

use crate::db::users::get_access_logs;

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
    let staff_name = email.split('@').next().unwrap_or("Admin").to_string();

    let mut ctx = Context::new();
    ctx.insert("specific_role", "admin");
    ctx.insert("email",         &email);
    ctx.insert("staff_name",    &staff_name);
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

    let email      = session.get::<String>("email").unwrap_or_default().unwrap_or_default();
    let staff_name = email.split('@').next().unwrap_or("Admin").to_string();
    let today      = chrono::Local::now().date_naive();

    let total_patients: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM patient")
        .fetch_one(pool.get_ref()).await.unwrap_or(0);

    let total_appointments: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM appointment")
        .fetch_one(pool.get_ref()).await.unwrap_or(0);

    let appts_today: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM appointment WHERE date = $1")
        .bind(today).fetch_one(pool.get_ref()).await.unwrap_or(0);

    let appts_this_month: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM appointment WHERE date_trunc('month', date) = date_trunc('month', $1::date)"
    ).bind(today).fetch_one(pool.get_ref()).await.unwrap_or(0);

    let appts_completed: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM appointment WHERE status = 'completed'"
    ).fetch_one(pool.get_ref()).await.unwrap_or(0);

    let appts_cancelled: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM appointment WHERE status IN ('cancelled', 'no_show')"
    ).fetch_one(pool.get_ref()).await.unwrap_or(0);

    let total_revenue: f64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(total_amount), 0)::float8 FROM bills WHERE payment_status = 'paid'"
    ).fetch_one(pool.get_ref()).await.unwrap_or(0.0_f64);

    let revenue_this_month: f64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(total_amount), 0)::float8 FROM bills WHERE payment_status = 'paid' AND date_trunc('month', created_at) = date_trunc('month', $1::timestamptz)"
    ).bind(chrono::Local::now()).fetch_one(pool.get_ref()).await.unwrap_or(0.0_f64);

    let outstanding_bills: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM bills WHERE payment_status = 'unpaid'"
    ).fetch_one(pool.get_ref()).await.unwrap_or(0);

    let total_prescriptions: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM prescription")
        .fetch_one(pool.get_ref()).await.unwrap_or(0);

    let total_doctors: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE role = 'doctor'")
        .fetch_one(pool.get_ref()).await.unwrap_or(0);

    let total_nurses: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE role = 'nurse'")
        .fetch_one(pool.get_ref()).await.unwrap_or(0);

    let total_receptionists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE role = 'receptionist'")
        .fetch_one(pool.get_ref()).await.unwrap_or(0);

    let mut ctx = Context::new();
    ctx.insert("specific_role",       "admin");
    ctx.insert("email",               &email);
    ctx.insert("staff_name",          &staff_name);
    ctx.insert("total_patients",      &total_patients);
    ctx.insert("total_appointments",  &total_appointments);
    ctx.insert("appts_today",         &appts_today);
    ctx.insert("appts_this_month",    &appts_this_month);
    ctx.insert("appts_completed",     &appts_completed);
    ctx.insert("appts_cancelled",     &appts_cancelled);
    ctx.insert("total_revenue",       &format!("{:.2}", total_revenue));
    ctx.insert("revenue_this_month",  &format!("{:.2}", revenue_this_month));
    ctx.insert("outstanding_bills",   &outstanding_bills);
    ctx.insert("total_prescriptions", &total_prescriptions);
    ctx.insert("total_doctors",       &total_doctors);
    ctx.insert("total_nurses",        &total_nurses);
    ctx.insert("total_receptionists", &total_receptionists);

    match tmpl.render("admin/analytics.html", &ctx) {
        Ok(html) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
        Err(e)   => HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
    }
}
