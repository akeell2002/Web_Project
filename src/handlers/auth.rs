use actix_web::{web, HttpResponse, Responder};
use actix_session::Session;
use tera::{Context, Tera};
use sqlx::PgPool;
use crate::models::{LoginForm, RegisterForm, LoginResponse};
use crate::db::users::{create_user, authenticate_user};

// Staff login page
pub async fn staff_login(tera: web::Data<Tera>) -> impl Responder {
    let ctx = Context::new();

    match tera.render("staff/login.html", &ctx) {
        Ok(html_content) => HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(html_content),
        Err(e) => {
            println!("Template error: {}", e);
            HttpResponse::InternalServerError().body("Failed to compile layout.")
        }
    }
}

// Patient login page
pub async fn patient_login(tera: web::Data<Tera>) -> impl Responder {
    let ctx = Context::new();

    match tera.render("patient/login.html", &ctx) {
        Ok(html_content) => HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(html_content),
        Err(e) => {
            println!("Template error: {}", e);
            HttpResponse::InternalServerError().body("Failed to compile layout.")
        }
    }
}

// Process login form submission for both staff and patients under same function
pub async fn login(
    pool: web::Data<PgPool>,
    form: web::Form<LoginForm>,
    session: Session,
) -> impl Responder {
    match authenticate_user(&pool, &form.username, &form.password).await {
        Ok(Some(user)) => {
            // Store user id in session
            let _ = session.insert("user_id", user.id);
            let _ = session.insert("username", &user.username);
            let _ = session.insert("role", &user.role);
            
            HttpResponse::SeeOther()
                .append_header(("Location", "/patient/dashboard"))
                .finish()
        }
        Ok(None) => HttpResponse::Unauthorized().json(LoginResponse {
            success: false,
            message: "Invalid username or password".to_string(),
            role: None,
        }),
        Err(e) => HttpResponse::InternalServerError().json(LoginResponse {
            success: false,
            message: format!("Error: {}", e),
            role: None,
        }),
    }
}

// Patient registration page
pub async fn show_register(tera: web::Data<Tera>) -> impl Responder {
    let ctx = Context::new();
    match tera.render("patient/register.html", &ctx) {
        Ok(html) => HttpResponse::Ok().content_type("text/html").body(html),
        Err(e) => HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
    }
}

// Process registration form submission
pub async fn register(
    pool: web::Data<PgPool>,
    form: web::Form<RegisterForm>,
    session: Session,
) -> impl Responder {
    match create_user(&pool, &form, "patient").await {
        Ok(user) => {
            let _ = session.insert("user_id", user.id);
            let _ = session.insert("username", &user.username);
            let _ = session.insert("role", &user.role);
            
            HttpResponse::SeeOther()
                .append_header(("Location", "/patient/dashboard"))
                .finish()
        }
        Err(e) => HttpResponse::BadRequest().json(LoginResponse {
            success: false,
            message: e,
            role: None,
        }),
    }
}

// Dashboard page after login
pub async fn dashboard(
    session: Session,
    tera: web::Data<Tera>,
) -> impl Responder {
    match session.get::<i32>("user_id") {
        Ok(Some(user_id)) => {
            let username: String = session.get("username").unwrap().unwrap_or("Unknown".to_string());
            let mut ctx = Context::new();
            ctx.insert("user_id", &user_id);
            ctx.insert("username", &username);
            match tera.render("dashboard.html", &ctx) {
                Ok(html) => HttpResponse::Ok().content_type("text/html").body(html),
                Err(e) => HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
            }
        }
        _ => {
            HttpResponse::SeeOther()
                .append_header(("Location", "/patient/login"))
                .finish()
        }
    }
}

// Logout
pub async fn logout(session: Session) -> impl Responder {
    session.clear();
    HttpResponse::SeeOther()
    .append_header(("Location", "/patient/login"))
        .finish()
}