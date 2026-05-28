use actix_web::{web, HttpResponse, Responder};
use actix_session::Session;
use tera::{Context, Tera};
use sqlx::PgPool;

use crate::models::user::UserRole;
use crate::models::staff::CreateStaffProfile;
use crate::db::staff::register_staff;

// Form payload matching the Admin Onboarding Dashboard form
#[derive(Debug, serde::Deserialize)]
pub struct StaffOnboardForm {
    pub email: String,
    pub password: String,
    pub role_selection: String, // "doctor", "nurse", or "receptionist"
    pub first_name: String,
    pub last_name: String,
    pub phone_number: Option<String>,
}

/// Process a request from the Admin to add a new staff member
pub async fn add_staff(
    pool: web::Data<PgPool>,
    session: Session,
    form: web::Form<StaffOnboardForm>,
) -> impl Responder {
    // 1. Guard check: Ensure only an Admin can invoke this action
    if let Ok(Some(role)) = session.get::<String>("role") {
        if role != "admin" {
            return HttpResponse::Forbidden().body("Access Denied: Administrative privileges required.");
        }
    } else {
        return HttpResponse::Unauthorized().body("Session expired or missing.");
    }

    // 2. Parse string choice back into our database model Enum
    let targeted_role = match form.role_selection.as_str() {
        "doctor" => UserRole::Doctor,
        "nurse" => UserRole::Nurse,
        "receptionist" => UserRole::Receptionist,
        _ => return HttpResponse::BadRequest().body("Invalid staff role selection."),
    };

    let profile = CreateStaffProfile {
        first_name: form.first_name.clone(),
        last_name: form.last_name.clone(),
        phone_number: form.phone_number.clone(),
    };

    // 3. Execute the database transaction
    match register_staff(&pool, &form.email, &form.password, targeted_role, profile).await {
        Ok(_) => {
            // Redirect back to the admin dashboard with a success state
            HttpResponse::SeeOther()
                .append_header(("Location", "/admin/dashboard?success=staff_created"))
                .finish()
        }
        Err(e) => HttpResponse::InternalServerError().body(format!("Onboarding process failed: {}", e)),
    }
}