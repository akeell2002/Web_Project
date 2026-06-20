use actix_web::HttpResponse;
use actix_session::Session;
use crate::models::user::UserRole;

// ── Shared admin helpers ──────────────────────────────────────────────────────

pub(super) fn admin_only(session: &Session) -> Result<(), HttpResponse> {
    match session.get::<String>("role") {
        Ok(Some(role)) if role == "admin" => Ok(()),
        Ok(Some(_)) => Err(HttpResponse::Forbidden().body("Access Denied: Admin access required.")),
        _ => Err(HttpResponse::SeeOther().append_header(("Location", "/staff/login")).finish()),
    }
}

pub(super) fn parse_role(role: &str) -> Result<UserRole, HttpResponse> {
    match role.trim().to_lowercase().as_str() {
        "admin"        => Ok(UserRole::Admin),
        "doctor"       => Ok(UserRole::Doctor),
        "nurse"        => Ok(UserRole::Nurse),
        "receptionist" => Ok(UserRole::Receptionist),
        _ => Err(HttpResponse::BadRequest().body("Invalid staff role selected.")),
    }
}

pub(super) fn staff_role_title(role: Option<&UserRole>) -> (&'static str, &'static str) {
    match role {
        Some(UserRole::Admin)        => ("Admins",          "admin"),
        Some(UserRole::Doctor)       => ("Doctors",         "doctor"),
        Some(UserRole::Nurse)        => ("Nurses",          "nurse"),
        Some(UserRole::Receptionist) => ("Receptionists",   "receptionist"),
        _ =>                            ("All Staff",        "all"),
    }
}

pub(super) fn parse_directory_role(role: Option<&str>) -> Result<Option<UserRole>, HttpResponse> {
    match role.map(|v| v.trim().to_lowercase()) {
        None => Ok(None),
        Some(v) if v.is_empty() || v == "all" => Ok(None),
        Some(v) => match v.as_str() {
            "admin"        => Ok(Some(UserRole::Admin)),
            "doctor"       => Ok(Some(UserRole::Doctor)),
            "nurse"        => Ok(Some(UserRole::Nurse)),
            "receptionist" => Ok(Some(UserRole::Receptionist)),
            _ => Err(HttpResponse::BadRequest().body("Invalid role filter supplied.")),
        },
    }
}

// ── Submodules ────────────────────────────────────────────────────────────────

mod staff;
mod monitoring;
mod patients;
mod support;

pub use staff::*;
pub use monitoring::*;
pub use patients::*;
pub use support::*;
