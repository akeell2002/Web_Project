use actix_web::{web, HttpResponse, Responder, HttpRequest};
use actix_session::Session;
use tera::{Context, Tera};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::user::{LoginForm, UserRole};
use crate::db::users::{authenticate_user, log_access_event};
use super::{OtpStore, issue_otp};

// Handler for the staff login page
pub async fn staff_login(tera: web::Data<Tera>) -> impl Responder {
    let ctx = Context::new();
    match tera.render("staff/login.html", &ctx) {
        Ok(html) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
        Err(e) => HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
    }
}

// Handler for the patient login page
pub async fn patient_login(tera: web::Data<Tera>) -> impl Responder {
    let ctx = Context::new();
    match tera.render("patient/login.html", &ctx) {
        Ok(html) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
        Err(e) => HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
    }
}

// Handler for processing login form submissions
pub async fn login(
    pool:    web::Data<PgPool>,
    tera:    web::Data<Tera>,
    req:       HttpRequest,
    form:      web::Form<LoginForm>,
    session:   Session,
    otp_store: web::Data<OtpStore>,
) -> impl Responder {
    match authenticate_user(&pool, &form.email, &form.password).await {
        Ok(Some(user)) => {
            // Patient login via patient portal only, and staff login via staff portal only
            let is_patient_portal = req.path().starts_with("/patient");
            let is_patient_role   = user.role == UserRole::Patient;
            if is_patient_portal != is_patient_role {
                let mut ctx = Context::new();
                ctx.insert("email",          &form.email);
                ctx.insert("error_message",  &"Invalid email or password.");
                let template = if is_patient_portal { "patient/login.html" } else { "staff/login.html" };
                return match tera.render(template, &ctx) {
                    Ok(html) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
                    Err(e)   => HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
                };
            }

            // Only admim use 2FA
            if user.role == UserRole::Admin {
                issue_otp(otp_store.get_ref(), user.id, &user.email);
                let _ = session.insert("pending_2fa_user_id", user.id);
                let _ = session.insert("pending_2fa_email", &user.email);

                if let Err(err) = log_access_event(
                    pool.get_ref(),
                    Some(user.id),
                    Some(&user.email),
                    "login_2fa_challenge",
                    Some(user.id),
                    &user.email,
                    "admin",
                    &format!("{} passed password check; 2FA code sent.", user.email),
                )
                .await
                {
                    eprintln!("Security log write failed for login_2fa_challenge: {}", err);
                }

                return HttpResponse::SeeOther()
                    .append_header(("Location", "/admin/verify-otp"))
                    .finish();
            }

            let _ = session.insert("user_id", user.id);
            let _ = session.insert("email", &user.email);

            let role_str = match user.role {
                UserRole::Admin        => "admin",
                UserRole::Doctor       => "doctor",
                UserRole::Nurse        => "nurse",
                UserRole::Receptionist => "receptionist",
                UserRole::Patient      => "patient",
            };
            let _ = session.insert("role", role_str);

            // Cache the user's real name in the session so every page can show it without an extra DB query
            let name: String = match user.role {
                UserRole::Patient => {
                    sqlx::query_scalar::<_, String>(
                        "SELECT first_name || ' ' || last_name FROM patient WHERE id = $1"
                    )
                    .bind(user.id)
                    .fetch_one(pool.get_ref())
                    .await
                    .unwrap_or_else(|_| user.email.split('@').next().unwrap_or("Patient").to_string())
                }
                _ => {
                    sqlx::query_scalar::<_, String>(
                        "SELECT first_name || ' ' || last_name FROM staff WHERE id = $1"
                    )
                    .bind(user.id)
                    .fetch_one(pool.get_ref())
                    .await
                    .unwrap_or_else(|_| user.email.split('@').next().unwrap_or("Staff").to_string())
                }
            };
            let _ = session.insert("name", &name);

            if let Err(err) = log_access_event(
                pool.get_ref(),
                Some(user.id),
                Some(&user.email),
                "login_success",
                Some(user.id),
                &user.email,
                role_str,
                &format!("{} logged in successfully.", user.email),
            )
            .await
            {
                eprintln!("Security log write failed for login_success: {}", err);
            }

            let redirect_target = match user.role {
                UserRole::Admin                                              => "/admin/dashboard?success=login",
                UserRole::Doctor | UserRole::Nurse | UserRole::Receptionist => "/staff/dashboard?success=login",
                UserRole::Patient                                            => "/patient/dashboard?success=login",
            };

            HttpResponse::SeeOther().append_header(("Location", redirect_target)).finish()
        }
        Ok(None) => {
            let mut ctx = Context::new();
            ctx.insert("error_message", &"Invalid email or password.");
            ctx.insert("email", &form.email);

            let path = req.path();
            let template_name = if path.starts_with("/patient") {
                "patient/login.html"
            } else {
                "staff/login.html"
            };

            match tera.render(template_name, &ctx) {
                Ok(html) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
                Err(e)   => HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
            }
        }
        Err(e) => HttpResponse::InternalServerError().body(format!("System Authentication Error: {}", e)),
    }
}

// Handler for logging out users
pub async fn logout(pool: web::Data<PgPool>, session: Session) -> impl Responder {
    let current_user_id = session.get::<Uuid>("user_id").unwrap_or_default();
    let current_email   = session.get::<String>("email").unwrap_or_default().unwrap_or_default();
    let current_role    = session.get::<String>("role").unwrap_or_default().unwrap_or_default();

    if !current_email.is_empty() {
        let role_label = match current_role.as_str() {
            "admin"        => "admin",
            "doctor"       => "doctor",
            "nurse"        => "nurse",
            "receptionist" => "receptionist",
            "patient"      => "patient",
            _              => "unknown",
        };

        if let Err(err) = log_access_event(
            pool.get_ref(),
            current_user_id,
            Some(&current_email),
            "logout_success",
            current_user_id,
            &current_email,
            role_label,
            &format!("{} logged out successfully.", current_email),
        )
        .await
        {
            eprintln!("Security log write failed for logout_success: {}", err);
        }
    }

    session.clear();

    if current_role == "patient" {
        HttpResponse::SeeOther().append_header(("Location", "/patient/login")).finish()
    } else {
        HttpResponse::SeeOther().append_header(("Location", "/staff/login")).finish()
    }
}