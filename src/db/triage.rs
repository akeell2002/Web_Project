// src/db/triage.rs
use sqlx::PgPool;
use uuid::Uuid;
use chrono::Local;
use serde_json::{Value, json};

/// Fetches patients who are checked in and waiting for the Nurse
pub async fn get_triage_queue(pool: &PgPool) -> Result<Vec<Value>, String> {
    let today_date = Local::now().date_naive();

    let rows = sqlx::query!(
        r#"
        SELECT a.id, a.queue_number, a.start_time,
               p.first_name, p.last_name, p.gender, p.date_of_birth
        FROM appointment a
        JOIN patient p ON a.patient_id = p.id
        WHERE a.date = $1 AND a.status = 'checked_in'
        ORDER BY a.queue_number ASC
        "#,
        today_date
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to fetch triage queue: {}", e))?;

    let mut list = Vec::new();
    for row in rows {
        list.push(json!({
            "id": row.id,
            "time": row.start_time.format("%I:%M %p").to_string(),
            "queue_number": row.queue_number.unwrap_or(0),
            "patient_name": format!("{} {}", row.first_name, row.last_name),
            "gender": row.gender.unwrap_or_else(|| "N/A".to_string()),
            "dob": row.date_of_birth.to_string(),
        }));
    }
    Ok(list)
}

/// Records the patient's vitals and upgrades their status to 'vitals_taken'
pub async fn record_patient_vitals(
    pool: &PgPool,
    appointment_id: Uuid,
    nurse_id: Uuid,
    bp: String,
    temp: String, 
    weight: String,
    height: String,
) -> Result<(), String> {
    // Database Transaction: If one fails, everything rolls back safely.
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    // 1. Insert the vitals
    sqlx::query!(
        r#"
        INSERT INTO triage_vitals (appointment_id, nurse_id, blood_pressure, temperature, weight_kg, height_cm)
        VALUES ($1, $2, $3, $4::text::numeric, $5::text::numeric, $6::text::numeric)
        "#,
        appointment_id, nurse_id, bp, temp, weight, height
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Failed to insert vitals: {}", e))?;

    // 2. Update the appointment status
    sqlx::query!(
        "UPDATE appointment SET status = 'vitals_taken' WHERE id = $1",
        appointment_id
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Failed to update status: {}", e))?;

    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(())
}