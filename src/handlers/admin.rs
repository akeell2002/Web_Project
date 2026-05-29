use actix_web::{web, HttpResponse, Responder};
use actix_session::Session;
use tera::{Tera, Context};
use sqlx::PgPool;
use crate::models::user::UserRole;
use uuid::Uuid;
use serde::Deserialize;
use crate::models::staff::{CreateStaffProfile, OnboardStaffForm};
use crate::db::staff::{get_staff_directory, register_staff};
use crate::db::security::get_access_logs;

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

    let ctx = Context::new();
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

    let mut ctx = Context::new();
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
    let mut ctx = Context::new();
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

fn staff_only(session: &Session) -> Result<(), HttpResponse> {
    match session.get::<String>("role") {
        Ok(Some(role)) if (role == "doctor" || role == "nurse" || role == "receptionist" || role == "admin") => Ok(()),
        Ok(Some(_)) => Err(HttpResponse::Forbidden().body("Access Denied: Staff access required.")),
        _ => Err(HttpResponse::SeeOther().append_header(("Location", "/staff/login")).finish()),
    }
}

pub async fn patient_directory_page(
    pool: web::Data<PgPool>,
    session: Session,
    tmpl: web::Data<Tera>,
) -> impl Responder {
    if let Err(response) = staff_only(&session) {
        return response;
    }

    let patients = match crate::db::patients::get_patient_directory(&pool).await {
        Ok(rows) => rows,
        Err(err_msg) => return HttpResponse::InternalServerError().body(format!("Failed to load patients: {}", err_msg)),
    };

    let mut ctx = Context::new();
    ctx.insert("patients", &patients);

    match tmpl.render("staff/patient_directory.html", &ctx) {
        Ok(html) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
        Err(e) => HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
    }
}