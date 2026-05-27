use sqlx::PgPool;
use chrono::NaiveDateTime;
use crate::models::appointment::{Appointment, CreateAppointmentDto};

// Fetch all appointments
pub async fn get_all_appointments(pool: &PgPool) -> Result<Vec<Appointment>, sqlx::Error> {
    let appointments = sqlx::query_as!(
        Appointment,
        r#"
        SELECT 
            id, 
            patient_id as "patient_id!", 
            doctor_id as "doctor_id!", 
            appointment_date, 
            status, 
            reason, 
            created_at as "created_at!"
        FROM appointments 
        ORDER BY appointment_date ASC
        "#
    )
    .fetch_all(pool)
    .await?;

    Ok(appointments)
}

// Create a new appointment
pub async fn create_appointment(pool: &PgPool, data: CreateAppointmentDto) -> Result<Appointment, sqlx::Error> {
    let parsed_date = NaiveDateTime::parse_from_str(&data.appointment_date, "%Y-%m-%dT%H:%M")
        .expect("Invalid date format from form");

    let appointment = sqlx::query_as!(
        Appointment,
        r#"
        INSERT INTO appointments (patient_id, doctor_id, appointment_date, status, reason)
        VALUES ($1, $2, $3, 'scheduled', $4)
        RETURNING 
            id, 
            patient_id as "patient_id!", 
            doctor_id as "doctor_id!", 
            appointment_date, 
            status, 
            reason, 
            created_at as "created_at!"
        "#,
        data.patient_id,
        data.doctor_id,
        parsed_date,
        data.reason
    )
    .fetch_one(pool)
    .await?;

    Ok(appointment)
}

// Fetch appointments for a specific patient
pub async fn get_appointments_by_patient(pool: &sqlx::PgPool, patient_id: i32) -> Result<Vec<Appointment>, sqlx::Error> {
    let appointments = sqlx::query_as!(
        Appointment,
        r#"
        SELECT 
            id, 
            patient_id as "patient_id!", 
            doctor_id as "doctor_id!", 
            appointment_date, 
            status, 
            reason, 
            created_at as "created_at!"
        FROM appointments 
        WHERE patient_id = $1
        ORDER BY appointment_date DESC
        "#,
        patient_id
    )
    .fetch_all(pool)
    .await?;

    Ok(appointments)
}
