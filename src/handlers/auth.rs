use actix_web::{web, HttpResponse, Responder};
use tera::{Context, Tera};
use sqlx::PgPool;
use crate::models::{LoginForm, RegisterForm, LoginResponse};
use crate::db::users::{create_user, authenticate_user};

// Show login page
pub async fn show_login(tera: web::Data<Tera>) -> impl Responder {
    let ctx = Context::new();
    match tera.render("auth/login.html", &ctx) {
        Ok(html) => HttpResponse::Ok().content_type("text/html").body(html),
        Err(e) => HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
    }
}

// Process login form submission
pub async fn login(
    pool: web::Data<PgPool>,
    form: web::Form<LoginForm>,
) -> impl Responder {
    match authenticate_user(&pool, &form.username, &form.password).await {
        Ok(Some(user)) => {
            // Returns success only, i havent do full yet
            HttpResponse::Ok().json(LoginResponse {
                success: true,
                message: format!("Welcome back, {}!", user.username),
                role: Some(user.role),
            })
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

// Show registration page
pub async fn show_register(tera: web::Data<Tera>) -> impl Responder {
    let ctx = Context::new();
    match tera.render("auth/register.html", &ctx) {
        Ok(html) => HttpResponse::Ok().content_type("text/html").body(html),
        Err(e) => HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
    }
}

// Process registration form submission
pub async fn register(
    pool: web::Data<PgPool>,
    form: web::Form<RegisterForm>,
) -> impl Responder {
    // Default role is patient for new signup
    match create_user(&pool, &form, "patient").await {
        Ok(user) => HttpResponse::Ok().json(LoginResponse {
            success: true,
            message: format!("Account created! Welcome, {}!", user.username),
            role: Some(user.role),
        }),
        Err(e) => HttpResponse::BadRequest().json(LoginResponse {
            success: false,
            message: e,
            role: None,
        }),
    }
}