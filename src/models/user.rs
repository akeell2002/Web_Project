use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};

// Enum representing the different roles a user can have in the system, converted from Postgres ENUM to Rust Enum
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "user_role", rename_all = "lowercase")]
pub enum UserRole {
    Admin,
    Doctor,
    Nurse,
    Receptionist,
    Patient,
}

// Struct for pulling user data from database
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    #[serde(skip_serializing)] // To avoid sending password hash in API responses, safety stuff
    pub password: String,
    pub role: UserRole,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Struct representing the form data for registering a new patient
#[derive(Debug, serde::Deserialize)]
pub struct PatientRegisterForm {
    pub email: String,
    pub password: String,  
    pub confirm_password: String,
    pub first_name: String,
    pub last_name: String,
    pub date_of_birth: String, 
    pub gender: Option<String>,
    pub phone_number: Option<String>,
    pub emergency_contact_name: Option<String>,
    pub emergency_contact_phone: Option<String>,
}

// Struct representing the form data for logging in a user
#[derive(Debug, serde::Deserialize)]
pub struct LoginForm {
    pub email: String,
    pub password: String,
}

// Struct representing log entries for user actions in the system
#[derive(Debug, Clone, Serialize)]
pub struct AccessLogEntry {
    pub id: Uuid,
    pub actor_user_id: Option<Uuid>,
    pub actor_email: Option<String>,
    pub action_type: String,
    pub action_label: String,
    pub action_kind: String,
    pub target_user_id: Option<Uuid>,
    pub target_email: String,
    pub target_role: String,
    pub details: String,
    pub created_at: String,
}

// Helper function to convert action types into labels
fn humanize_action(action_type: &str) -> String {
    match action_type {
        "staff_account_created"   => "Staff Created".to_string(),
        "staff_account_updated"   => "Staff Updated".to_string(),
        "staff_account_deleted"   => "Staff Deleted".to_string(),
        "patient_account_created" => "Patient Created".to_string(),
        "patient_account_updated" => "Patient Updated".to_string(),
        "patient_account_deleted" => "Patient Deleted".to_string(),
        "admin_created"           => "Admin Created".to_string(),
        "doctor_created"          => "Doctor Created".to_string(),
        "nurse_created"           => "Nurse Created".to_string(),
        "receptionist_created"    => "Receptionist Created".to_string(),
        "seed_account_created"    => "Seed Account Created".to_string(),
        "staff_created"           => "Staff Created".to_string(),
        "patient_checked_in"      => "Patient Checked In".to_string(),
        "patient_admitted"        => "Patient Admitted".to_string(),
        "patient_discharged"      => "Patient Discharged".to_string(),
        // Fallback if none of the above match just in case
        other => other
            .split('_')
            .map(|w| {
                let mut chars = w.chars();
                match chars.next() {
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                    None => String::new(),
                }
            })
            .collect::<Vec<_>>()
            .join(" "),
    }
}

// Helper function to determine the colour group for the action badge.
fn action_colour(action_type: &str) -> &'static str {
    if action_type.ends_with("_deleted") { "deleted" }
    else if action_type.ends_with("_updated") { "updated" }
    else if action_type.ends_with("_created") { "created" }
    else if action_type.contains("admitted") { "admitted" }
    else if action_type.contains("discharged") { "discharged" }
    else { "access" }
}

// Implementation block for AccessLogEntry to create a new instance from its parts
impl AccessLogEntry {
    pub fn from_parts(
        id: Uuid,
        actor_user_id: Option<Uuid>,
        actor_email: Option<String>,
        action_type: String,
        target_user_id: Option<Uuid>,
        target_email: String,
        target_role: String,
        details: String,
        created_at: DateTime<Utc>,
    ) -> Self {
        let action_label = humanize_action(&action_type);
        let action_kind = action_colour(&action_type).to_string();
        Self {
            id,
            actor_user_id,
            actor_email,
            action_type,
            action_label,
            action_kind,
            target_user_id,
            target_email,
            target_role,
            details,
            created_at: created_at.format("%Y-%m-%d %H:%M").to_string(),
        }
    }
}