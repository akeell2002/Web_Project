use actix_web::{web, HttpResponse, Responder};
use actix_session::Session;
use tera::{Context, Tera};
use sqlx::PgPool;
use uuid::Uuid;

pub async fn patient_profile_page(
    pool:    web::Data<PgPool>,
    session: Session,
    tera:    web::Data<Tera>,
) -> impl Responder {
    let role = session.get::<String>("role").unwrap_or_default().unwrap_or_default();
    if role != "patient" {
        return HttpResponse::SeeOther().append_header(("Location", "/patient/login")).finish();
    }

    let email      = session.get::<String>("email").unwrap_or_default().unwrap_or_default();
    let patient_id = match session.get::<Uuid>("user_id").unwrap_or_default() {
        Some(id) => id,
        None     => return HttpResponse::SeeOther().append_header(("Location", "/patient/login")).finish(),
    };
    let staff_name = email.split('@').next().unwrap_or("Patient").to_string();

    match crate::db::patients::get_patient_detail(&pool, patient_id).await {
        Ok(Some(profile)) => {
            let mut ctx = Context::new();
            ctx.insert("specific_role", "patient");
            ctx.insert("email",         &email);
            ctx.insert("staff_name",    &staff_name);
            ctx.insert("profile",       &profile);
            match tera.render("patient/my_profile.html", &ctx) {
                Ok(html) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
                Err(e)   => HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Profile not found."),
        Err(e)   => HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    }
}

pub async fn staff_profile_page(
    pool:    web::Data<PgPool>,
    session: Session,
    tera:    web::Data<Tera>,
) -> impl Responder {
    let role = session.get::<String>("role").unwrap_or_default().unwrap_or_default();
    let valid_roles = ["admin", "doctor", "nurse", "receptionist"];
    if !valid_roles.contains(&role.as_str()) {
        return HttpResponse::SeeOther().append_header(("Location", "/staff/login")).finish();
    }

    let email   = session.get::<String>("email").unwrap_or_default().unwrap_or_default();
    let user_id = match session.get::<Uuid>("user_id").unwrap_or_default() {
        Some(id) => id,
        None     => return HttpResponse::SeeOther().append_header(("Location", "/staff/login")).finish(),
    };
    let staff_name = email.split('@').next().unwrap_or("Staff").to_string();

    match crate::db::staff::get_staff_profile(&pool, user_id).await {
        Ok(Some(profile)) => {
            let mut ctx = Context::new();
            ctx.insert("specific_role", &role);
            ctx.insert("email",         &email);
            ctx.insert("staff_name",    &staff_name);
            ctx.insert("profile",       &profile);
            match tera.render("staff/my_profile.html", &ctx) {
                Ok(html) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
                Err(e)   => HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Profile not found."),
        Err(e)   => HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    }
}
