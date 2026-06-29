use actix_web::{web, HttpResponse, Responder};
use actix_session::Session;
use tera::{Context, Tera};
use sqlx::PgPool;
use uuid::Uuid;
use chrono::{Duration, Utc};
use rand::Rng;
use serde::Deserialize;

use super::{OtpStore, OtpEntry};
use crate::db::users::log_access_event;

const OTP_TTL_MINUTES: i64 = 1; // OTP valid for 1 minute
const MAX_OTP_ATTEMPTS: u8 = 5; // Maximum number of attempts before OTP expires

// Struct for deserializing the OTP form submission
#[derive(Deserialize)]
pub struct OtpForm {
    pub code: String,
}

// Handler to issue a new OTP for admin in terminal
pub fn issue_otp(store: &OtpStore, user_id: Uuid, email: &str) {
    let code = format!("{:06}", rand::thread_rng().gen_range(0..1_000_000));
    let entry = OtpEntry {
        code: code.clone(),
        email: email.to_string(),
        expires_at: Utc::now() + Duration::minutes(OTP_TTL_MINUTES),
        attempts: 0,
    };
    {
        let mut s = store.lock().unwrap();
        s.insert(user_id, entry);
    }
    send_otp_email(email, &code);
}

// Handler to send the OTP code to the terminal for now
fn send_otp_email(email: &str, code: &str) {
    eprintln!(
        "\n[MOCK EMAIL] Admin 2FA verification code for {}:\n  Your code is: {}  (valid for {} minute(s))\n",
        email, code, OTP_TTL_MINUTES
    );
}

// To verify the OTP code submitted by the admin user
pub async fn verify_otp_page(tera: web::Data<Tera>, session: Session) -> impl Responder {
    if session.get::<Uuid>("pending_2fa_user_id").ok().flatten().is_none() {
        return HttpResponse::SeeOther()
            .append_header(("Location", "/staff/login"))
            .finish();
    }
    let mut ctx = Context::new();
    if let Ok(Some(email)) = session.get::<String>("pending_2fa_email") {
        ctx.insert("email", &email);
    }
    render(&tera, &ctx)
}

// To handle the OTP submission
pub async fn submit_otp(
    pool:    web::Data<PgPool>,
    tera:    web::Data<Tera>,
    store:   web::Data<OtpStore>,
    session: Session,
    form:    web::Form<OtpForm>,
) -> impl Responder {
    // Must be in the middle of 2FA, otherwise back to login
    let pending_id = match session.get::<Uuid>("pending_2fa_user_id").ok().flatten() {
        Some(id) => id,
        None => {
            return HttpResponse::SeeOther()
                .append_header(("Location", "/staff/login"))
                .finish();
        }
    };

    let submitted = form.code.trim().to_string();

    enum Outcome { Verified(String), Wrong, Expired }

    let outcome = {
        let mut s = store.lock().unwrap();
        match s.get_mut(&pending_id) {
            None => Outcome::Expired,
            Some(entry) => {
                if Utc::now() > entry.expires_at || entry.attempts >= MAX_OTP_ATTEMPTS {
                    s.remove(&pending_id);
                    Outcome::Expired
                } else if entry.code == submitted {
                    let email = entry.email.clone();
                    s.remove(&pending_id);
                    Outcome::Verified(email)
                } else {
                    entry.attempts += 1;
                    Outcome::Wrong
                }
            }
        }
    };

    match outcome {
        Outcome::Verified(email) => {
            // Successful verification and log the admin in
            let name: String = sqlx::query_scalar::<_, String>(
                "SELECT first_name || ' ' || last_name FROM staff WHERE id = $1",
            )
            .bind(pending_id)
            .fetch_one(pool.get_ref())
            .await
            .unwrap_or_else(|_| email.split('@').next().unwrap_or("Admin").to_string());

            session.remove("pending_2fa_user_id");
            session.remove("pending_2fa_email");
            let _ = session.insert("user_id", pending_id);
            let _ = session.insert("email", &email);
            let _ = session.insert("role", "admin");
            let _ = session.insert("name", &name);

            if let Err(err) = log_access_event(
                pool.get_ref(),
                Some(pending_id),
                Some(&email),
                "login_2fa_success",
                Some(pending_id),
                &email,
                "admin",
                &format!("{} passed 2FA verification.", email),
            )
            .await
            {
                eprintln!("Security log write failed for login_2fa_success: {}", err);
            }

            HttpResponse::SeeOther()
                .append_header(("Location", "/admin/dashboard?success=login"))
                .finish()
        }
        Outcome::Expired => {
            // Code gone, expired, or too many attempts — force a fresh login
            session.remove("pending_2fa_user_id");
            session.remove("pending_2fa_email");
            let mut ctx = Context::new();
            ctx.insert(
                "error",
                &"Your code expired or too many attempts were made. Please log in again.",
            );
            ctx.insert("expired", &true);
            render(&tera, &ctx)
        }
        Outcome::Wrong => {
            let mut ctx = Context::new();
            if let Ok(Some(email)) = session.get::<String>("pending_2fa_email") {
                ctx.insert("email", &email);
            }
            ctx.insert("error", &"Incorrect code. Please try again.");
            render(&tera, &ctx)
        }
    }
}

fn render(tera: &Tera, ctx: &Context) -> HttpResponse {
    match tera.render("auth/verify_otp.html", ctx) {
        Ok(html) => HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(html),
        Err(e) => HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
    }
}
