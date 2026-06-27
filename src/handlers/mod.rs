// To export for use in other modules
pub mod auth;
pub mod admin;
pub mod appointments;
pub mod patients;
pub mod billing;
pub mod beds;

use actix_session::Session;
use actix_web::HttpResponse;

// Utility function to retrieve the display name of the logged-in user from the session
pub fn get_display_name(session: &Session) -> String {
    if let Ok(Some(name)) = session.get::<String>("name") {
        if !name.is_empty() {
            return name;
        }
    }
    session.get::<String>("email")
        .unwrap_or_default()
        .unwrap_or_default()
        .split('@')
        .next()
        .unwrap_or("User")
        .to_string()
}

// Utility function to enforce staff-only access based on session role
pub(crate) fn staff_only(session: &Session) -> Result<(), HttpResponse> {
    match session.get::<String>("role") {
        Ok(Some(role)) if matches!(role.as_str(), "doctor" | "nurse" | "receptionist" | "admin") => Ok(()),
        Ok(Some(_)) => Err(HttpResponse::Forbidden().body("Access Denied: Staff access required.")),
        _ => Err(HttpResponse::SeeOther().append_header(("Location", "/staff/login")).finish()),
    }
}

