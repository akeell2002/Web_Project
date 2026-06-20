use actix_web::{web, HttpResponse, Responder};
use tera::{Context, Tera};
use sqlx::PgPool;
use uuid::Uuid;
use serde::Deserialize;

use super::ResetTokenStore;

#[derive(Deserialize)]
pub struct ForgotPasswordForm {
    pub email: String,
}

#[derive(Deserialize)]
pub struct ResetPasswordForm {
    pub token:            String,
    pub new_password:     String,
    pub confirm_password: String,
}

#[derive(Deserialize)]
pub struct ResetTokenQuery {
    pub token: Option<String>,
}

pub async fn forgot_password_page(tera: web::Data<Tera>) -> impl Responder {
    let ctx = Context::new();
    match tera.render("auth/forgot_password.html", &ctx) {
        Ok(html) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
        Err(e)   => HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
    }
}

pub async fn submit_forgot_password(
    pool:        web::Data<PgPool>,
    tera:        web::Data<Tera>,
    token_store: web::Data<ResetTokenStore>,
    form:        web::Form<ForgotPasswordForm>,
) -> impl Responder {
    match crate::db::users::find_user_by_email(pool.get_ref(), &form.email).await {
        Ok(Some(_)) => {
            let token = Uuid::new_v4().to_string();
            {
                let mut store = token_store.lock().unwrap();
                store.insert(token.clone(), form.email.clone());
            }
            eprintln!(
                "\n[MOCK EMAIL] Password reset link for {}:\n  http://127.0.0.1:8080/reset-password?token={}\n",
                form.email, token
            );
        }
        _ => {} // Don't reveal whether the email exists
    }

    let mut ctx = Context::new();
    ctx.insert("email", &form.email);
    match tera.render("auth/forgot_password_sent.html", &ctx) {
        Ok(html) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
        Err(e)   => HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
    }
}

pub async fn reset_password_page(
    tera:        web::Data<Tera>,
    token_store: web::Data<ResetTokenStore>,
    query:       web::Query<ResetTokenQuery>,
) -> impl Responder {
    let token = match &query.token {
        Some(t) => t.clone(),
        None    => return HttpResponse::BadRequest().body("Missing reset token."),
    };

    let valid = {
        let store = token_store.lock().unwrap();
        store.contains_key(&token)
    };

    let mut ctx = Context::new();
    if !valid {
        ctx.insert("error", "This reset link is invalid or has already been used.");
    } else {
        ctx.insert("token", &token);
    }

    match tera.render("auth/reset_password.html", &ctx) {
        Ok(html) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
        Err(e)   => HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
    }
}

pub async fn submit_reset_password(
    pool:        web::Data<PgPool>,
    tera:        web::Data<Tera>,
    token_store: web::Data<ResetTokenStore>,
    form:        web::Form<ResetPasswordForm>,
) -> impl Responder {
    if form.new_password != form.confirm_password {
        let mut ctx = Context::new();
        ctx.insert("token", &form.token);
        ctx.insert("error", "Passwords do not match.");
        return match tera.render("auth/reset_password.html", &ctx) {
            Ok(html) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
            Err(e)   => HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
        };
    }

    let email = {
        let mut store = token_store.lock().unwrap();
        store.remove(&form.token)
    };

    match email {
        None => HttpResponse::BadRequest().body("Invalid or expired reset token."),
        Some(email) => {
            match crate::db::users::update_user_password(pool.get_ref(), &email, &form.new_password).await {
                Ok(true) => HttpResponse::SeeOther()
                    .append_header(("Location", "/staff/login?success=password_reset"))
                    .finish(),
                _ => HttpResponse::InternalServerError().body("Failed to update password."),
            }
        }
    }
}
