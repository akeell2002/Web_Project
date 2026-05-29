use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::models::user::UserRole;

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

#[derive(Debug, serde::Deserialize)]
pub struct OnboardStaffForm {
    pub email: String,
    pub password: String,
    pub first_name: String,
    pub last_name: String,
    pub phone_number: Option<String>,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct StaffDashboardCounts {
    pub total_staff: i64,
    pub admins: i64,
    pub doctors: i64,
    pub nurses: i64,
    pub receptionists: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct StaffDirectoryRow {
    pub id: Uuid,
    pub email: String,
    pub role: UserRole,
    pub display_name: String,
    pub phone_number: Option<String>,
    pub created_at: String,
}