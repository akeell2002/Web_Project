use actix_web::{web, HttpResponse, Responder};
use actix_session::Session;
use tera::{Tera, Context};
use sqlx::PgPool;
use crate::models::user::UserRole;
use uuid::Uuid;
use serde::Deserialize;
use crate::models::staff::{CreateStaffProfile, OnboardStaffForm};
use crate::db::staff::{get_staff_directory, register_staff};
use crate::db::users::get_access_logs;

fn admin_only(session: &Session) -> Result<(), HttpResponse> {
    match session.get::<String>("role") {
        Ok(Some(role)) if role == "admin" => Ok(()),
        Ok(Some(_)) => Err(HttpResponse::Forbidden().body("Access Denied: Admin access required.")),
        _ => Err(HttpResponse::SeeOther().append_header(("Location", "/staff/login")).finish()),
    }
}

fn build_staff_profile(first_name: &str, last_name: &str, phone_number: Option<String>) -> CreateStaffProfile {
    CreateStaffProfile {
        first_name: first_name.to_string(),
        last_name: last_name.to_string(),
        phone_number,
    }
}

fn parse_role(role: &str) -> Result<UserRole, HttpResponse> {
    match role.trim().to_lowercase().as_str() {
        "admin" => Ok(UserRole::Admin),
        "doctor" => Ok(UserRole::Doctor),
        "nurse" => Ok(UserRole::Nurse),
        "receptionist" => Ok(UserRole::Receptionist),
        _ => Err(HttpResponse::BadRequest().body("Invalid staff role selected.")),
    }
}

fn staff_role_title(role: Option<&UserRole>) -> (&'static str, &'static str) {
    match role {
        Some(UserRole::Admin) => ("Admins", "admin"),
        Some(UserRole::Doctor) => ("Doctors", "doctor"),
        Some(UserRole::Nurse) => ("Nurses", "nurse"),
        Some(UserRole::Receptionist) => ("Receptionists", "receptionist"),
        _ => ("All Staff", "all"),
    }
}

fn parse_directory_role(role: Option<&str>) -> Result<Option<UserRole>, HttpResponse> {
    match role.map(|value| value.trim().to_lowercase()) {
        None => Ok(None),
        Some(value) if value.is_empty() || value == "all" => Ok(None),
        Some(value) => match value.as_str() {
            "admin" => Ok(Some(UserRole::Admin)),
            "doctor" => Ok(Some(UserRole::Doctor)),
            "nurse" => Ok(Some(UserRole::Nurse)),
            "receptionist" => Ok(Some(UserRole::Receptionist)),
            _ => Err(HttpResponse::BadRequest().body("Invalid role filter supplied.")),
        },
    }
}

#[derive(Deserialize)]
pub struct StaffDirectoryQuery {
    pub role: Option<String>,
}

/// GET request handler for the unified staff onboarding page
pub async fn onboard_staff_page(tmpl: web::Data<Tera>, session: Session) -> impl Responder {
    if let Err(response) = admin_only(&session) {
        return response;
    }

    let email = session.get::<String>("email").unwrap_or_default().unwrap_or_default();
    let staff_name = email.split('@').next().unwrap_or("Admin").to_string();

    let mut ctx = Context::new();
    ctx.insert("specific_role", "admin");
    ctx.insert("email", &email);
    ctx.insert("staff_name", &staff_name);

    match tmpl.render("admin/onboard_staff.html", &ctx) {
        Ok(html) => HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .append_header(("Cache-Control", "no-store, no-cache, must-revalidate, max-age=0"))
            .body(html),
        Err(e) => HttpResponse::InternalServerError().body(format!("Template compilation error: {}", e)),
    }
}

/// POST the form submission for onboarding a new staff member
pub async fn onboard_staff_submit(
    pool: web::Data<PgPool>,
    session: Session,
    form: web::Form<OnboardStaffForm>,
) -> impl Responder {
    if let Err(response) = admin_only(&session) {
        return response;
    }

    let role = match parse_role(&form.role) {
        Ok(role) => role,
        Err(response) => return response,
    };

    let created_by_user_id = session.get::<Uuid>("user_id").unwrap_or_default();
    let created_by_email = session.get::<String>("email").unwrap_or_default();

    let staff_profile = build_staff_profile(&form.first_name, &form.last_name, form.phone_number.clone());

    match register_staff(
        &pool,
        &form.email,
        &form.password,
        role,
        staff_profile,
        created_by_user_id,
        created_by_email.as_deref(),
    )
    .await
    {
        Ok(user) => {
            let success_key = match user.role {
                UserRole::Admin => "admin_added",
                UserRole::Doctor => "doctor_added",
                UserRole::Nurse => "nurse_added",
                UserRole::Receptionist => "receptionist_added",
                UserRole::Patient => "staff_added",
            };

            HttpResponse::SeeOther()
                .append_header(("Location", format!("/admin/dashboard?success={}", success_key)))
                .finish()
        }
        Err(err_msg) => HttpResponse::BadRequest().body(format!("Failed to onboard staff member: {}", err_msg)),
    }
}

pub async fn security_monitoring_page(
    pool: web::Data<PgPool>,
    session: Session,
    tmpl: web::Data<Tera>,
) -> impl Responder {
    if let Err(response) = admin_only(&session) {
        return response;
    }

    let access_logs = match get_access_logs(&pool, 100).await {
        Ok(logs) => logs,
        Err(err_msg) => {
            return HttpResponse::InternalServerError().body(format!("Failed to load access logs: {}", err_msg));
        }
    };

    let email = session.get::<String>("email").unwrap_or_default().unwrap_or_default();
    let staff_name = email.split('@').next().unwrap_or("Admin").to_string();

    let mut ctx = Context::new();
    ctx.insert("specific_role", "admin");
    ctx.insert("email", &email);
    ctx.insert("staff_name", &staff_name);
    ctx.insert("access_logs", &access_logs);
    ctx.insert("log_count", &access_logs.len());

    match tmpl.render("admin/security.html", &ctx) {
        Ok(html) => HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .append_header(("Cache-Control", "no-store, no-cache, must-revalidate, max-age=0"))
            .body(html),
        Err(err) => HttpResponse::InternalServerError().body(format!("Template compilation error: {}", err)),
    }
}

pub async fn staff_directory_page(
    pool: web::Data<PgPool>,
    session: Session,
    query: web::Query<StaffDirectoryQuery>,
    tmpl: web::Data<Tera>,
) -> impl Responder {
    if let Err(response) = admin_only(&session) {
        return response;
    }

    let role_filter = match parse_directory_role(query.role.as_deref()) {
        Ok(role) => role,
        Err(response) => return response,
    };

    let staff_members = match get_staff_directory(&pool, role_filter.clone()).await {
        Ok(staff_members) => staff_members,
        Err(err_msg) => {
            return HttpResponse::InternalServerError().body(format!("Failed to load staff directory: {}", err_msg));
        }
    };

    let (title, role_label) = staff_role_title(role_filter.as_ref());

    let email = session.get::<String>("email").unwrap_or_default().unwrap_or_default();
    let staff_name = email.split('@').next().unwrap_or("Admin").to_string();

    let mut ctx = Context::new();
    ctx.insert("specific_role", "admin");
    ctx.insert("email", &email);
    ctx.insert("staff_name", &staff_name);
    ctx.insert("directory_title", &title);
    ctx.insert("selected_role", &role_label);
    ctx.insert("staff_members", &staff_members);
    ctx.insert("staff_count", &staff_members.len());

    match tmpl.render("admin/staff_directory.html", &ctx) {
        Ok(html) => HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .append_header(("Cache-Control", "no-store, no-cache, must-revalidate, max-age=0"))
            .body(html),
        Err(err) => HttpResponse::InternalServerError().body(format!("Template compilation error: {}", err)),
    }
}

pub async fn patient_directory_page(
    pool: web::Data<PgPool>,
    session: Session,
    tmpl: web::Data<Tera>,
) -> impl Responder {
    if let Err(response) = super::staff_only(&session) {
        return response;
    }

    let patients = match crate::db::patients::get_patient_directory(&pool).await {
        Ok(rows) => rows,
        Err(err_msg) => return HttpResponse::InternalServerError().body(format!("Failed to load patients: {}", err_msg)),
    };

    let current_role = session.get::<String>("role").unwrap_or_default().unwrap_or_default();
    let email = session.get::<String>("email").unwrap_or_default().unwrap_or_default();
    let staff_name = email.split('@').next().unwrap_or("Staff").to_string();

    let mut ctx = Context::new();
    ctx.insert("patients", &patients);
    ctx.insert("specific_role", &current_role);
    ctx.insert("email", &email);
    ctx.insert("staff_name", &staff_name);

    match tmpl.render("staff/patient_directory.html", &ctx) {
        Ok(html) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
        Err(e) => HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
    }
}

pub async fn analytics_page(
    pool: web::Data<PgPool>,
    session: Session,
    tmpl: web::Data<Tera>,
) -> impl Responder {
    if let Err(response) = admin_only(&session) {
        return response;
    }

    let email = session.get::<String>("email").unwrap_or_default().unwrap_or_default();
    let staff_name = email.split('@').next().unwrap_or("Admin").to_string();

    let today = chrono::Local::now().date_naive();

    // Total patients
    let total_patients: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM patient")
        .fetch_one(pool.get_ref()).await.unwrap_or(0);

    // Total appointments
    let total_appointments: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM appointment")
        .fetch_one(pool.get_ref()).await.unwrap_or(0);

    // Appointments today
    let appts_today: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM appointment WHERE date = $1")
        .bind(today)
        .fetch_one(pool.get_ref()).await.unwrap_or(0);

    // Appointments this month
    let appts_this_month: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM appointment WHERE date_trunc('month', date) = date_trunc('month', $1::date)"
    )
    .bind(today)
    .fetch_one(pool.get_ref()).await.unwrap_or(0);

    // Completed appointments
    let appts_completed: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM appointment WHERE status = 'completed'"
    )
    .fetch_one(pool.get_ref()).await.unwrap_or(0);

    // Cancelled / no-show
    let appts_cancelled: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM appointment WHERE status IN ('cancelled', 'no_show')"
    )
    .fetch_one(pool.get_ref()).await.unwrap_or(0);

    // Total revenue (all paid bills)
    let total_revenue: f64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(total_amount), 0) FROM bills WHERE payment_status = 'paid'"
    )
    .fetch_one(pool.get_ref()).await.unwrap_or(0.0_f64);

    // Revenue this month
    let revenue_this_month: f64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(total_amount), 0) FROM bills WHERE payment_status = 'paid' AND date_trunc('month', created_at) = date_trunc('month', $1::timestamptz)"
    )
    .bind(chrono::Local::now())
    .fetch_one(pool.get_ref()).await.unwrap_or(0.0_f64);

    // Outstanding (unpaid) bills
    let outstanding_bills: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM bills WHERE payment_status = 'unpaid'"
    )
    .fetch_one(pool.get_ref()).await.unwrap_or(0);

    // Total prescriptions
    let total_prescriptions: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM prescription")
        .fetch_one(pool.get_ref()).await.unwrap_or(0);

    // Staff counts
    let total_doctors: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM users WHERE role = 'doctor'"
    ).fetch_one(pool.get_ref()).await.unwrap_or(0);

    let total_nurses: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM users WHERE role = 'nurse'"
    ).fetch_one(pool.get_ref()).await.unwrap_or(0);

    let total_receptionists: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM users WHERE role = 'receptionist'"
    ).fetch_one(pool.get_ref()).await.unwrap_or(0);

    let mut ctx = Context::new();
    ctx.insert("specific_role", "admin");
    ctx.insert("email", &email);
    ctx.insert("staff_name", &staff_name);
    ctx.insert("total_patients", &total_patients);
    ctx.insert("total_appointments", &total_appointments);
    ctx.insert("appts_today", &appts_today);
    ctx.insert("appts_this_month", &appts_this_month);
    ctx.insert("appts_completed", &appts_completed);
    ctx.insert("appts_cancelled", &appts_cancelled);
    ctx.insert("total_revenue", &format!("{:.2}", total_revenue));
    ctx.insert("revenue_this_month", &format!("{:.2}", revenue_this_month));
    ctx.insert("outstanding_bills", &outstanding_bills);
    ctx.insert("total_prescriptions", &total_prescriptions);
    ctx.insert("total_doctors", &total_doctors);
    ctx.insert("total_nurses", &total_nurses);
    ctx.insert("total_receptionists", &total_receptionists);

    match tmpl.render("admin/analytics.html", &ctx) {
        Ok(html) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
        Err(e) => HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
    }
}

// --- Moved from handlers/receptionist.rs ---

// ── SUPPORT TICKETS (Public + Receptionist) ──────────────────────────────────

#[derive(serde::Deserialize)]
pub struct SupportTicketForm {
    pub submitter_name: String,
    pub submitter_email: String,
    pub issue_description: String,
}

#[derive(serde::Deserialize)]
pub struct ReplyForm {
    pub ticket_id: uuid::Uuid,
    pub reply_notes: String,
}

#[derive(serde::Deserialize)]
pub struct SupportQuery {
    pub sent: Option<String>,
}

/// GET /support — public contact form
pub async fn support_form_page(
    tmpl: web::Data<Tera>,
    query: web::Query<SupportQuery>,
) -> impl Responder {
    let mut ctx = Context::new();
    let sent = query.sent.as_deref().unwrap_or("") == "1";
    ctx.insert("sent", &sent);
    match tmpl.render("support/form.html", &ctx) {
        Ok(html) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
        Err(e) => HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
    }
}

/// POST /support/submit — anonymous ticket submission
pub async fn submit_support_ticket(
    pool: web::Data<PgPool>,
    form: web::Form<SupportTicketForm>,
) -> impl Responder {
    match crate::db::support::submit_ticket(
        &pool,
        &form.submitter_name,
        &form.submitter_email,
        &form.issue_description,
    ).await {
        Ok(_) => HttpResponse::SeeOther()
            .append_header(("Location", "/support?sent=1"))
            .finish(),
        Err(e) => HttpResponse::InternalServerError()
            .body(format!("Failed to submit ticket: {}", e)),
    }
}

#[derive(serde::Deserialize)]
pub struct SupportDashQuery {
    pub replied: Option<String>,
}

/// GET /admin/support — admin support ticket dashboard
pub async fn support_dashboard(
    pool: web::Data<PgPool>,
    session: Session,
    tmpl: web::Data<Tera>,
    query: web::Query<SupportDashQuery>,
) -> impl Responder {
    match session.get::<String>("role") {
        Ok(Some(role)) if role == "admin" => {},
        Ok(Some(_)) => return HttpResponse::Forbidden().body("Access Denied"),
        _ => return HttpResponse::SeeOther().append_header(("Location", "/staff/login")).finish(),
    }

    let email = session.get::<String>("email").unwrap_or_default().unwrap_or_default();
    let staff_name = email.split('@').next().unwrap_or("Staff").to_string();
    let current_role = session.get::<String>("role").unwrap_or_default().unwrap_or_default();

    let tickets = match crate::db::support::get_all_tickets(&pool).await {
        Ok(t) => t,
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };

    let mut ctx = Context::new();
    ctx.insert("tickets", &tickets);
    ctx.insert("email", &email);
    ctx.insert("staff_name", &staff_name);
    ctx.insert("specific_role", &current_role);
    let replied = query.replied.as_deref().unwrap_or("") == "1";
    ctx.insert("replied", &replied);

    match tmpl.render("admin/support_dashboard.html", &ctx) {
        Ok(html) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
        Err(e) => HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
    }
}

/// POST /admin/support/reply — admin sends reply
pub async fn submit_reply(
    pool: web::Data<PgPool>,
    session: Session,
    form: web::Form<ReplyForm>,
) -> impl Responder {
    match session.get::<String>("role") {
        Ok(Some(role)) if role == "admin" => {},
        Ok(Some(_)) => return HttpResponse::Forbidden().body("Access Denied"),
        _ => return HttpResponse::SeeOther().append_header(("Location", "/staff/login")).finish(),
    }

    match crate::db::support::reply_to_ticket(&pool, form.ticket_id, &form.reply_notes).await {
        Ok(_) => HttpResponse::SeeOther()
            .append_header(("Location", "/admin/support?replied=1"))
            .finish(),
        Err(e) => HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    }
}