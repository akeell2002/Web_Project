use sqlx::PgPool;
use crate::models::patient::{Patient, PatientForm};
use chrono::NaiveDate;

pub async fn create_patient(
    pool: &PgPool, 
    form: &PatientForm, 
    user_id: i32
) -> Result<i32, sqlx::Error> {
    
    // Convert the HTML date string (YYYY-MM-DD) into a native Rust/Postgres Date
    let dob = NaiveDate::parse_from_str(&form.date_of_birth, "%Y-%m-%d")
        .unwrap_or_default();

    let record = sqlx::query!(
        r#"
        INSERT INTO patients (first_name, last_name, date_of_birth, gender, phone, email, address, blood_type, created_by)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING id
        "#,
        form.first_name,
        form.last_name,
        dob,
        form.gender,
        form.phone,
        form.email,
        form.address,
        form.blood_type,
        user_id
    )
    .fetch_one(pool)
    .await?;

    Ok(record.id)
}

pub async fn get_all_patients(pool: &PgPool) -> Result<Vec<Patient>, sqlx::Error> {
    let patients = sqlx::query_as!(
        Patient,
        r#"
        SELECT id, first_name, last_name, date_of_birth, gender, phone, email, address, blood_type, created_by, created_at 
        FROM patients
        "#
    )
    .fetch_all(pool)
    .await?;

    Ok(patients)
}