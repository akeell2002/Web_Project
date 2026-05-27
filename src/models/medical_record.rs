use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use chrono::NaiveDateTime;

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct MedicalRecord {
    pub id: i32,
    pub patient_id: Option<i32>,
    pub doctor_id: Option<i32>,
    pub diagnosis: String,
    pub prescription: Option<String>,
    pub notes: Option<String>, 
    pub record_date: Option<NaiveDateTime>, 
}

#[derive(Debug, Deserialize)]
pub struct CreateMedicalRecordDto {
    pub patient_id: i32,
    pub doctor_id: i32,
    pub diagnosis: String,
    pub prescription: Option<String>,
    pub notes: Option<String>,
}