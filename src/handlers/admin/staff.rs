use actix_web::{web, HttpResponse, Responder};
use actix_session::Session;
use tera::{Tera, Context};
use sqlx::PgPool;
use uuid::Uuid;
use serde::Deserialize;

use crate::models::user::UserRole;
use crate::models::staff::{CreateStaffProfile, OnboardStaffForm};
use crate::db::staff::{get_staff_directory, register_staff};

use super::{admin_only, parse_role, parse_directory_role, staff_role_title};

#[derive(Deserialize)]
pub struct StaffDirectoryQuery {
    pub role: Option<String>,
}

pub async fn onboard_staff_page(tmpl: web::Data<Tera>, session: Session) -> impl Responder {
    if let Err(response) = admin_only(&session) {
        return response;
    }

    let email      = session.get::<String>("email").unwrap_or_default().unwrap_or_default();
    let display_name = crate::handlers::get_display_name(&session);

    let mut ctx = Context::new();
    ctx.insert("specific_role", "admin");
    ctx.insert("email",         &email);
    ctx.insert("display_name", &display_name);

    match tmpl.render("admin/onboard_staff.html", &ctx) {
        Ok(html) => HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .append_header(("Cache-Control", "no-store, no-cache, must-revalidate, max-age=0"))
            .body(html),
        Err(e) => HttpResponse::InternalServerError().body(format!("Template compilation error: {}", e)),
    }
}

pub async fn onboard_staff_submit(
    pool:    web::Data<PgPool>,
    session: Session,
    form:    web::Form<OnboardStaffForm>,
    tmpl:    web::Data<Tera>,
) -> impl Responder {
    if let Err(response) = admin_only(&session) {
        return response;
    }

    let role = match parse_role(&form.role) {
        Ok(r)         => r,
        Err(response) => return response,
    };

    let created_by_user_id = session.get::<Uuid>("user_id").unwrap_or_default();
    let created_by_email   = session.get::<String>("email").unwrap_or_default();

    let staff_profile = CreateStaffProfile {
        first_name:   form.first_name.clone(),
        last_name:    form.last_name.clone(),
        phone_number: form.phone_number.clone(),
    };

    match register_staff(
        &pool, &form.email, &form.password, role, staff_profile,
        created_by_user_id, created_by_email.as_deref(),
    )
    .await
    {
        Ok(user) => {
            let success_key = match user.role {
                UserRole::Admin        => "admin_added",
                UserRole::Doctor       => "doctor_added",
                UserRole::Nurse        => "nurse_added",
                UserRole::Receptionist => "receptionist_added",
                UserRole::Patient      => "staff_added",
            };
            HttpResponse::SeeOther()
                .append_header(("Location", format!("/admin/dashboard?success={}", success_key)))
                .finish()
        }
        Err(err_msg) => {
            // Show a friendly inline error instead of a raw 400 page
            let friendly = if err_msg.contains("duplicate key") || err_msg.contains("unique constraint") {
                format!("An account with the email '{}' already exists. Please use a different email.", form.email)
            } else {
                format!("Failed to create account: {}", err_msg)
            };

            let email        = session.get::<String>("email").unwrap_or_default().unwrap_or_default();
            let display_name = crate::handlers::get_display_name(&session);

            let mut ctx = Context::new();
            ctx.insert("specific_role",  "admin");
            ctx.insert("email",          &email);
            ctx.insert("display_name",   &display_name);
            ctx.insert("error_message",  &friendly);

            match tmpl.render("admin/onboard_staff.html", &ctx) {
                Ok(html) => HttpResponse::Ok()
                    .content_type("text/html; charset=utf-8")
                    .body(html),
                Err(e) => HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
            }
        }
    }
}

pub async fn staff_directory_page(
    pool:    web::Data<PgPool>,
    session: Session,
    query:   web::Query<StaffDirectoryQuery>,
    tmpl:    web::Data<Tera>,
) -> impl Responder {
    if let Err(response) = admin_only(&session) {
        return response;
    }

    let role_filter = match parse_directory_role(query.role.as_deref()) {
        Ok(r)         => r,
        Err(response) => return response,
    };

    let staff_members = match get_staff_directory(&pool, role_filter.clone()).await {
        Ok(s)        => s,
        Err(err_msg) => return HttpResponse::InternalServerError().body(format!("Failed to load staff directory: {}", err_msg)),
    };

    let (title, role_label) = staff_role_title(role_filter.as_ref());

    let email      = session.get::<String>("email").unwrap_or_default().unwrap_or_default();
    let display_name = crate::handlers::get_display_name(&session);

    let mut ctx = Context::new();
    ctx.insert("specific_role",    "admin");
    ctx.insert("email",            &email);
    ctx.insert("display_name", &display_name);
    ctx.insert("directory_title",  &title);
    ctx.insert("selected_role",    &role_label);
    ctx.insert("staff_members",    &staff_members);
    ctx.insert("staff_count",      &staff_members.len());

    match tmpl.render("admin/staff_directory.html", &ctx) {
        Ok(html) => HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .append_header(("Cache-Control", "no-store, no-cache, must-revalidate, max-age=0"))
            .body(html),
        Err(err) => HttpResponse::InternalServerError().body(format!("Template compilation error: {}", err)),
    }
}
