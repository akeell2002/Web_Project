use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Staff {
    pub id: Uuid, // Links directly to user id
    pub first_name: String,
    pub last_name: String,
    pub phone_number: Option<String>,
    pub updated_at: DateTime<Utc>,
}

// Used by the Admin when onboarding a new doctor, nurse, or receptionist
#[derive(Debug, Deserialize)]
pub struct CreateStaffProfile {
    pub first_name: String,
    pub last_name: String,
    pub phone_number: Option<String>,
}