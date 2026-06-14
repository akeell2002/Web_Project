use sqlx::PgPool;
use uuid::Uuid;
use chrono::Local;
use serde_json::{Value, json};

/// Fetches the daily queue for a specific doctor
pub async fn get_doctor_queue(
    pool: &PgPool,
    doctor_id: Uuid,
) -> Result<Vec<Value>, String> {
    let today_date = Local::now().date_naive();

    // Notice the ::text casts on temperature, weight, and height!
    let rows = sqlx::query!(
        r#"
        SELECT a.id, a.queue_number, a.start_time, a.status::text as "status!",
               p.first_name, p.last_name, p.gender, p.date_of_birth,
               v.blood_pressure, 
               v.temperature::text as temperature, 
               v.weight_kg::text as weight_kg, 
               v.height_cm::text as height_cm
        FROM appointment a
        JOIN patient p ON a.patient_id = p.id
        LEFT JOIN triage_vitals v ON v.appointment_id = a.id
        WHERE a.doctor_id = $1 AND a.date = $2 AND a.status IN ('vitals_taken', 'checked_in')
        ORDER BY a.queue_number ASC
        "#,
        doctor_id, today_date
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to fetch doctor queue: {}", e))?;

    let mut list = Vec::new();
    for row in rows {
        list.push(json!({
            "id": row.id,
            "time": row.start_time.format("%I:%M %p").to_string(),
            "queue_number": row.queue_number.unwrap_or(0),
            "status": row.status,
            "patient_name": format!("{} {}", row.first_name, row.last_name),
            "gender": row.gender.unwrap_or_else(|| "N/A".to_string()),
            "dob": row.date_of_birth.to_string(),
            // Since they are already text, we don't need .to_string() anymore!
            "bp": row.blood_pressure.unwrap_or_else(|| "--".to_string()),
            "temp": row.temperature.unwrap_or_else(|| "--".to_string()),
            "weight": row.weight_kg.unwrap_or_else(|| "--".to_string()),
        }));
    }
    Ok(list)
}

/// Fetches details for a single active consultation session
pub async fn get_consultation_details(
    pool: &sqlx::PgPool,
    appointment_id: uuid::Uuid,
) -> Result<serde_json::Value, String> {
    let row = sqlx::query!(
        r#"
        SELECT a.id, a.queue_number, a.status::text as "status!",
               p.id as patient_id, p.first_name, p.last_name, p.gender, p.date_of_birth,
               v.blood_pressure, 
               v.temperature::text as temperature, 
               v.weight_kg::text as weight_kg, 
               v.height_cm::text as height_cm
        FROM appointment a
        JOIN patient p ON a.patient_id = p.id
        LEFT JOIN triage_vitals v ON v.appointment_id = a.id
        WHERE a.id = $1
        "#,
        appointment_id
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Failed to fetch consultation details: {}", e))?;

    match row {
        Some(r) => Ok(serde_json::json!({
            "appointment_id": r.id,
            "queue_number": r.queue_number.unwrap_or(0),
            "status": r.status,
            "patient_id": r.patient_id,
            "patient_name": format!("{} {}", r.first_name, r.last_name),
            "gender": r.gender.unwrap_or_else(|| "N/A".to_string()),
            "dob": r.date_of_birth.to_string(),
            "bp": r.blood_pressure.unwrap_or_else(|| "--".to_string()),
            "temp": r.temperature.unwrap_or_else(|| "--".to_string()),
            "weight": r.weight_kg.unwrap_or_else(|| "--".to_string()),
            "height": r.height_cm.unwrap_or_else(|| "--".to_string()),
        })),
        None => Err("Consultation session not found.".to_string()),
    }
}