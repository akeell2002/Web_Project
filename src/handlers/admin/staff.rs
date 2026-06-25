use actix_web::{web, HttpResponse, Responder};
use actix_session::Session;
use tera::{Tera, Context};
use sqlx::PgPool;
use uuid::Uuid;
use serde::Deserialize;

use crate::models::user::UserRole;
use crate::models::staff::{CreateStaffProfile, OnboardStaffForm};
use crate::db::staff::{
    get_staff_directory, register_staff, get_staff_dashboard_counts,
    get_staff_profile, admin_update_staff, delete_staff,
};

use super::{admin_only, parse_role, parse_directory_role, staff_role_title};

#[derive(Deserialize)]
pub struct StaffDirectoryQuery {
    pub role: Option<String>,
}

#[derive(Deserialize)]
pub struct EditStaffForm {
    pub first_name:   String,
    pub last_name:    String,
    pub email:        String,
    pub phone_number: Option<String>,
    pub role:         String,
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
                UserRole::Admin        => "admin_created",
                UserRole::Doctor       => "doctor_created",
                UserRole::Nurse        => "nurse_created",
                UserRole::Receptionist => "receptionist_created",
                UserRole::Patient      => "staff_created",
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

    let counts = get_staff_dashboard_counts(&pool).await.unwrap_or_default();

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
    ctx.insert("counts",           &counts);

    match tmpl.render("admin/staff_directory.html", &ctx) {
        Ok(html) => HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .append_header(("Cache-Control", "no-store, no-cache, must-revalidate, max-age=0"))
            .body(html),
        Err(err) => HttpResponse::InternalServerError().body(format!("Template compilation error: {}", err)),
    }
}

/// GET /admin/staff/{id}/edit - show the staff edit form pre-filled
pub async fn show_edit_staff_page(
    pool:    web::Data<PgPool>,
    session: Session,
    tmpl:    web::Data<Tera>,
    path:    web::Path<Uuid>,
) -> impl Responder {
    if let Err(r) = admin_only(&session) { return r; }

    let staff_id     = path.into_inner();
    let email        = session.get::<String>("email").unwrap_or_default().unwrap_or_default();
    let display_name = crate::handlers::get_display_name(&session);

    match get_staff_profile(&pool, staff_id).await {
        Ok(Some(staff)) => {
            let mut ctx = Context::new();
            ctx.insert("specific_role", "admin");
            ctx.insert("email",         &email);
            ctx.insert("display_name",  &display_name);
            ctx.insert("staff",         &staff);
            match tmpl.render("admin/edit_staff.html", &ctx) {
                Ok(html) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
                Err(e)   => HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Staff member not found."),
        Err(e)   => HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    }
}

/// POST /admin/staff/{id}/edit - save updated staff account
pub async fn process_edit_staff(
    pool:    web::Data<PgPool>,
    session: Session,
    tmpl:    web::Data<Tera>,
    path:    web::Path<Uuid>,
    form:    web::Form<EditStaffForm>,
) -> impl Responder {
    if let Err(r) = admin_only(&session) { return r; }

    let staff_id    = path.into_inner();
    let admin_email = session.get::<String>("email").unwrap_or_default().unwrap_or_default();

    let role = match parse_role(&form.role) {
        Ok(r)  => r,
        Err(r) => return r,
    };

    match admin_update_staff(
        &pool, staff_id, &form.email, &form.first_name, &form.last_name,
        form.phone_number.clone(), role, Some(admin_email.as_str()),
    ).await {
        Ok(_) => HttpResponse::SeeOther()
            .append_header(("Location", "/admin/staff?success=staff_updated"))
            .finish(),
        Err(err_msg) => {
            // Re-render the edit form with the error and the values just entered.
            let staff = serde_json::json!({
                "id":         staff_id.to_string(),
                "email":      form.email,
                "role":       form.role,
                "first_name": form.first_name,
                "last_name":  form.last_name,
                "phone":      form.phone_number,
                "full_name":  format!("{} {}", form.first_name, form.last_name),
            });
            let mut ctx = Context::new();
            ctx.insert("specific_role", "admin");
            ctx.insert("email",         &admin_email);
            ctx.insert("display_name",  &crate::handlers::get_display_name(&session));
            ctx.insert("staff",         &staff);
            ctx.insert("error_message", &err_msg);
            match tmpl.render("admin/edit_staff.html", &ctx) {
                Ok(html) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
                Err(e)   => HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
            }
        }
    }
}

/// POST /admin/staff/{id}/delete - remove a staff account
pub async fn process_delete_staff(
    pool:    web::Data<PgPool>,
    session: Session,
    path:    web::Path<Uuid>,
) -> impl Responder {
    if let Err(r) = admin_only(&session) { return r; }

    let staff_id    = path.into_inner();
    let admin_email = session.get::<String>("email").unwrap_or_default().unwrap_or_default();

    // Guard: an admin cannot delete their own account.
    if session.get::<Uuid>("user_id").unwrap_or(None) == Some(staff_id) {
        return HttpResponse::SeeOther()
            .append_header(("Location", "/admin/staff?error=self_delete"))
            .finish();
    }

    let row = sqlx::query!(
        r#"SELECT email, role::text as "role!" FROM users WHERE id = $1 AND role <> 'patient'::user_role"#,
        staff_id
    )
    .fetch_optional(pool.get_ref())
    .await;

    let (target_email, target_role) = match row {
        Ok(Some(r)) => (r.email, r.role),
        Ok(None)    => return HttpResponse::NotFound().body("Staff account not found."),
        Err(e)      => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };

    match delete_staff(&pool, staff_id, &admin_email, &target_email, &target_role).await {
        Ok(_)  => HttpResponse::SeeOther()
            .append_header(("Location", "/admin/staff?success=staff_deleted"))
            .finish(),
        Err(e) => HttpResponse::InternalServerError().body(format!("Delete failed: {}", e)),
    }
}
