use uuid::Uuid;
use chrono::{NaiveDate, NaiveTime, DateTime, Utc};
use serde::{Deserialize, Serialize};

// Struct representing an appointment in the system
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

// Struct representing an appointment slot for UI
#[derive(Debug, Serialize)]
pub struct UIAppointmentSlot {
    pub time_string: String,
    pub raw_time: NaiveTime,
    pub is_available: bool,
}

// Struct representing the form data for an encounter submission
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

    // Check if admit button pressed
    pub action: Option<String>,
}