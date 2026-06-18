// src/db/triage.rs
use sqlx::PgPool;
use uuid::Uuid;
use chrono::Local;
use serde_json::{Value, json};

pub async fn get_triage_queue(pool: &PgPool) -> Result<Vec<Value>, String> {
    let today_date = Local::now().date_naive();

    let rows = sqlx::query!(
        r#"
        SELECT a.id, a.queue_number, a.start_time,
               p.first_name, p.last_name, p.gender, p.date_of_birth,
               r.room_name as "room_name?"
        FROM appointment a
        JOIN patient p ON a.patient_id = p.id
        LEFT JOIN room r ON a.room_id = r.id   
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
            "room": row.room_name.unwrap_or_else(|| "Triage Waiting".to_string()), 
        }));
    }
    Ok(list)
}

pub async fn record_patient_vitals(
    pool: &PgPool,
    appointment_id: Uuid,
    nurse_id: Uuid,
    bp: String,
    temp: String, 
    weight: String,
    height: String,
) -> Result<(), String> {
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

    // 2. Find an available consultation room using correct schema
    let assigned_room = sqlx::query!(
        r#"
        SELECT id FROM room 
        WHERE room_type = 'consultation'
        AND id NOT IN (
            SELECT DISTINCT room_id FROM appointment WHERE status = 'vitals_taken' AND room_id IS NOT NULL
        )
        LIMIT 1
        "#
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| format!("Failed checking available clinic space: {}", e))?;

    // 3. Update appointment status AND assign the room ID if one was found
    if let Some(room) = assigned_room {
        sqlx::query!(
            r#"
            UPDATE appointment 
            SET status = 'vitals_taken', room_id = $1, updated_at = NOW() 
            WHERE id = $2
            "#,
            room.id,
            appointment_id
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to assign room and update status: {}", e))?;
    } else {
        sqlx::query!(
            "UPDATE appointment SET status = 'vitals_taken', updated_at = NOW() WHERE id = $1",
            appointment_id
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to update status: {}", e))?;
    }

    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(())
}