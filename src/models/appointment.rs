use uuid::Uuid;
use chrono::{NaiveDate, NaiveTime, DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Appointment {
    pub id: Uuid,
    pub patient_id: Uuid,
    pub doctor_id: Option<Uuid>,
    pub room_id: Option<Uuid>,
    pub status: String, 
    pub date: NaiveDate,
    pub start_time: NaiveTime,
    pub end_time: NaiveTime,
    pub queue_number: Option<i32>,
    pub check_in_time: Option<DateTime<Utc>>,
    pub created_by: Option<Uuid>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

/// Represents the data sent when the patient clicks a specific available slot card
#[derive(Debug, Deserialize)]
pub struct BookAppointmentForm {
    pub doctor_id: Uuid,
    pub date: NaiveDate,
    pub start_time: NaiveTime,
}

/// Represents a single 15-minute block calculated by the backend for the front-end grid
#[derive(Debug, Serialize)]
pub struct UIAppointmentSlot {
    pub time_string: String, // e.g., "09:15 AM"
    pub raw_time: NaiveTime,  // Passed back to the form action
    pub is_available: bool,   // Controls if it's clickable or grayed out
}

// --- Moved from models/consultation.rs ---

#[derive(Debug, Deserialize)]
pub struct EncounterForm {
    // Medical Record Fields
    pub symptoms: Option<String>,
    pub diagnosis: String,
    pub treatment_notes: Option<String>,

    // Prescription Fields
    pub medicine_name: Option<String>,
    pub dosage: Option<String>,
    pub frequency: Option<String>,
    pub duration: Option<String>,
    pub instructions: Option<String>,
}