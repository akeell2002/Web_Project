pub mod auth;
pub mod admin;
pub mod appointments;
pub mod patients;
pub mod billing;
pub mod beds;

use actix_session::Session;
use actix_web::HttpResponse;

/// Shared guard: allows any staff role (admin, doctor, nurse, receptionist)
pub(crate) fn staff_only(session: &Session) -> Result<(), HttpResponse> {
    match session.get::<String>("role") {
        Ok(Some(role)) if matches!(role.as_str(), "doctor" | "nurse" | "receptionist" | "admin") => Ok(()),
        Ok(Some(_)) => Err(HttpResponse::Forbidden().body("Access Denied: Staff access required.")),
        _ => Err(HttpResponse::SeeOther().append_header(("Location", "/staff/login")).finish()),
    }
}