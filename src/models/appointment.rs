use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Appointment {
    pub id: i32,
    pub patient_id: i32,
    pub doctor_id: i32,
    pub appointment_date: NaiveDateTime,
    pub status: String, 
    pub reason: Option<String>, 
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateAppointmentDto {
    pub patient_id: i32,
    pub doctor_id: i32,
    pub appointment_date: String, 
    pub reason: Option<String>, 
}