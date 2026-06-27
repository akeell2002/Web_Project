use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc, NaiveDate};

// Struct representing a patient in the system
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Patient {
    pub id: Uuid, // Links directly to users id
    pub first_name: String,
    pub last_name: String,
    pub date_of_birth: NaiveDate,
    pub gender: Option<String>,
    pub phone_number: Option<String>,
    pub emergency_contact_name: Option<String>,
    pub emergency_contact_phone: Option<String>,
    pub updated_at: DateTime<Utc>,
}

// Struct representing the form data for creating a new patient profile
#[derive(Debug, Deserialize)]
pub struct CreatePatientProfile {
    pub first_name: String,
    pub last_name: String,
    pub date_of_birth: NaiveDate,
    pub gender: Option<String>,
    pub phone_number: Option<String>,
    pub emergency_contact_name: Option<String>,
    pub emergency_contact_phone: Option<String>,
}