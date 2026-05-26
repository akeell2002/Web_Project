use serde::{Deserialize, Serialize};
use chrono::{NaiveDate, NaiveDateTime};

// Maps exactly to your Postgres `patients` table
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Patient {
    pub id: i32,
    pub first_name: String,
    pub last_name: String,
    pub date_of_birth: NaiveDate,
    pub gender: String,
    pub phone: String,
    pub email: Option<String>,      // Nullable in SQL
    pub address: Option<String>,    // Nullable in SQL
    pub blood_type: Option<String>, // Nullable in SQL
    pub created_by: Option<i32>,
    pub created_at: Option<NaiveDateTime>,
}

// Form struct to capture incoming HTML form inputs
#[derive(Debug, Deserialize)]
pub struct PatientForm {
    pub first_name: String,
    pub last_name: String,
    pub date_of_birth: String, // Comes in as a string from the HTML date picker
    pub gender: String,
    pub phone: String,
    pub email: Option<String>,
    pub address: Option<String>,
    pub blood_type: Option<String>,
}