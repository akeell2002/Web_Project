use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};

// Postgres ENUM to Rust Enum
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
    #[serde(skip_serializing)] // To avoid sending password hash in API responses
    pub password: String,
    pub role: UserRole,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

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

#[derive(Debug, serde::Deserialize)]
pub struct LoginForm {
    pub email: String,
    pub password: String,
}

// --- Moved from models/access_log.rs ---

#[derive(Debug, Clone, Serialize)]
pub struct AccessLogEntry {
    pub id: Uuid,
    pub actor_user_id: Option<Uuid>,
    pub actor_email: Option<String>,
    pub action_type: String,
    pub target_user_id: Option<Uuid>,
    pub target_email: String,
    pub target_role: String,
    pub details: String,
    pub created_at: String,
}

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
        Self {
            id,
            actor_user_id,
            actor_email,
            action_type,
            target_user_id,
            target_email,
            target_role,
            details,
            created_at: created_at.format("%Y-%m-%d %H:%M").to_string(),
        }
    }
}