use actix_web::{web, HttpResponse, Responder};
use actix_session::Session;
use tera::{Context, Tera};
use sqlx::PgPool;
use uuid::Uuid;
use serde::Deserialize;
use chrono::NaiveDate;

// Handler for the patient profile page
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
    let display_name = crate::handlers::get_display_name(&session);

    match crate::db::patients::get_patient_detail(&pool, patient_id).await {
        Ok(Some(profile)) => {
            let mut ctx = Context::new();
            ctx.insert("specific_role", "patient");
            ctx.insert("email",         &email);
            ctx.insert("display_name",  &display_name);
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

// Struct for the patient profile update form submission
#[derive(Deserialize)]
pub struct UpdatePatientProfileForm {
    pub first_name:              String,
    pub last_name:               String,
    pub date_of_birth:           NaiveDate,
    pub gender:                  Option<String>,
    pub phone_number:            Option<String>,
    pub emergency_contact_name:  Option<String>,
    pub emergency_contact_phone: Option<String>,
}

// Struct for the staff profile update form submission
#[derive(Deserialize)]
pub struct UpdateStaffProfileForm {
    pub first_name:   String,
    pub last_name:    String,
    pub phone_number: Option<String>,
}

// Handler for updating the patient profile
pub async fn update_patient_profile_handler(
    pool:    web::Data<PgPool>,
    session: Session,
    form:    web::Form<UpdatePatientProfileForm>,
) -> impl Responder {
    let role = session.get::<String>("role").unwrap_or_default().unwrap_or_default();
    if role != "patient" {
        return HttpResponse::Forbidden().body("Access Denied.");
    }
    let patient_id = match session.get::<Uuid>("user_id").unwrap_or_default() {
        Some(id) => id,
        None     => return HttpResponse::SeeOther().append_header(("Location", "/patient/login")).finish(),
    };
    let email = session.get::<String>("email").unwrap_or_default().unwrap_or_default();
    match crate::db::patients::update_patient_profile(
        &pool,
        &email,
        patient_id,
        &form.first_name,
        &form.last_name,
        form.date_of_birth,
        form.gender.clone(),
        form.phone_number.clone(),
        form.emergency_contact_name.clone(),
        form.emergency_contact_phone.clone(),
    ).await {
        Ok(_)  => {
            let new_name = format!("{} {}", form.first_name.trim(), form.last_name.trim());
            let _ = session.insert("name", new_name);
            HttpResponse::SeeOther()
                .append_header(("Location", "/patient/profile?success=updated"))
                .finish()
        },
        Err(e) => HttpResponse::InternalServerError().body(format!("Update failed: {}", e)),
    }
}

// Handler for updating the staff profile
pub async fn update_staff_profile_handler(
    pool:    web::Data<PgPool>,
    session: Session,
    form:    web::Form<UpdateStaffProfileForm>,
) -> impl Responder {
    let role = session.get::<String>("role").unwrap_or_default().unwrap_or_default();
    let valid = ["admin", "doctor", "nurse", "receptionist"];
    if !valid.contains(&role.as_str()) {
        return HttpResponse::Forbidden().body("Access Denied.");
    }
    let user_id = match session.get::<Uuid>("user_id").unwrap_or_default() {
        Some(id) => id,
        None     => return HttpResponse::SeeOther().append_header(("Location", "/staff/login")).finish(),
    };

    match crate::db::staff::update_staff_profile(
        &pool,
        user_id,
        &form.first_name,
        &form.last_name,
        form.phone_number.clone(),
    ).await {
        Ok(_)  => {
            let new_name = format!("{} {}", form.first_name.trim(), form.last_name.trim());
            let _ = session.insert("name", new_name);
            HttpResponse::SeeOther()
                .append_header(("Location", "/staff/profile?success=updated"))
                .finish()
        },
        Err(e) => HttpResponse::InternalServerError().body(format!("Update failed: {}", e)),
    }
}

// Handler for patient viewing their medical history
pub async fn patient_medical_history_page(
    pool:    web::Data<PgPool>,
    session: Session,
    tera:    web::Data<Tera>,
) -> impl Responder {
    let role = session.get::<String>("role").unwrap_or_default().unwrap_or_default();
    if role != "patient" {
        return HttpResponse::SeeOther().append_header(("Location", "/patient/login")).finish();
    }

    let email = session.get::<String>("email").unwrap_or_default().unwrap_or_default();
    let patient_id = match session.get::<Uuid>("user_id").unwrap_or_default() {
        Some(id) => id,
        None     => return HttpResponse::SeeOther().append_header(("Location", "/patient/login")).finish(),
    };
    let display_name = crate::handlers::get_display_name(&session);

    match crate::db::patients::get_patient_detail(&pool, patient_id).await {
        Ok(Some(profile)) => {
            // Extract the visits array from the full profile in JSON
            let visits = profile.get("visits").cloned().unwrap_or(serde_json::Value::Array(vec![]));
            let mut ctx = Context::new();
            ctx.insert("specific_role", "patient");
            ctx.insert("email",         &email);
            ctx.insert("display_name",  &display_name);
            ctx.insert("visits",        &visits);
            match tera.render("patient/medical_history.html", &ctx) {
                Ok(html) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
                Err(e)   => HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Profile not found."),
        Err(e)   => HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    }
}

// Handler for patient viewing their bill history
pub async fn patient_bill_history_page(
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
    let display_name = crate::handlers::get_display_name(&session);

    let bills = match crate::db::billing::get_patient_bills(&pool, patient_id).await {
        Ok(list) => list,
        Err(e) => {
            eprintln!("Bill history query failure: {}", e);
            Vec::new()
        }
    };

    let mut ctx = Context::new();
    ctx.insert("specific_role", "patient");
    ctx.insert("email",         &email);
    ctx.insert("display_name",  &display_name);
    ctx.insert("bills",         &bills);

    match tera.render("patient/bill_history.html", &ctx) {
        Ok(html) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
        Err(e)   => HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
    }
}

// Handler for staff viewing their profile page
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
    let display_name = crate::handlers::get_display_name(&session);

    match crate::db::staff::get_staff_profile(&pool, user_id).await {
        Ok(Some(profile)) => {
            let mut ctx = Context::new();
            ctx.insert("specific_role", &role);
            ctx.insert("email",         &email);
            ctx.insert("display_name",  &display_name);
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