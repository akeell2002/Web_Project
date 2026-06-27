use sqlx::PgPool;
use uuid::Uuid;

// Patients checked in today waiting for triage sorted by Dynamic Priority Algorithm
pub async fn get_triage_queue(pool: &PgPool) -> Result<Vec<serde_json::Value>, String> {
    let today = chrono::Local::now().date_naive();

    let rows = sqlx::query!(
        r#"
        SELECT a.id, a.queue_number, a.start_time, 
               a.priority_level, a.check_in_time, /* <-- FETCH THE PRIORITY */
               p.first_name, p.last_name, p.gender, p.date_of_birth,
               r.room_name as "room_name?"
        FROM appointment a
        JOIN patient p ON a.patient_id = p.id
        LEFT JOIN room r ON a.room_id  = r.id
        WHERE a.date = $1 AND a.status = 'checked_in'
        ORDER BY (
            /* DYNAMIC SCORING ALGORITHM */
            CASE a.priority_level
                WHEN 1 THEN 1000
                WHEN 2 THEN 500
                WHEN 3 THEN 200
                WHEN 4 THEN 50
                ELSE 10
            END
            +
            /* AGING: Add 1 point for every minute the patient has waited */
            COALESCE(EXTRACT(EPOCH FROM (NOW() - a.check_in_time)) / 60.0, 0)
        ) DESC
        "#,
        today
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to fetch triage queue: {}", e))?;

    Ok(rows
        .into_iter()
        .map(|row| {
            serde_json::json!({
                "id":           row.id,
                "time":         row.start_time.format("%I:%M %p").to_string(),
                "queue_number": row.queue_number.unwrap_or(0),
                "priority":     row.priority_level, /* <-- PASS TO HTML */
                "patient_name": format!("{} {}", row.first_name, row.last_name),
                "gender":       row.gender.unwrap_or_else(|| "N/A".to_string()),
                "dob":          row.date_of_birth.to_string(),
                "room":         row.room_name.unwrap_or_else(|| "Triage Waiting".to_string()),
            })
        })
        .collect())
}

// Record vitals, auto-assign a consultation room and advance status to vitals_taken
pub async fn record_patient_vitals(
    pool: &PgPool,
    appointment_id: Uuid,
    nurse_id: Uuid,
    bp: String,
    temp: String,
    weight: String,
    height: String,
    priority_level: i32,
) -> Result<(), String> {
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    sqlx::query!(
        r#"
        INSERT INTO triage_vitals
            (appointment_id, nurse_id, blood_pressure, temperature, weight_kg, height_cm)
        VALUES ($1, $2, $3, $4::text::numeric, $5::text::numeric, $6::text::numeric)
        "#,
        appointment_id,
        nurse_id,
        bp,
        temp,
        weight,
        height
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Failed to insert vitals: {}", e))?;

    // Try to assign a free consultation room, else just update the status to vitals_taken without a room assignment
    let free_room = sqlx::query!(
        r#"
        SELECT id FROM room
        WHERE room_type = 'consultation'
          AND id NOT IN (
              SELECT DISTINCT room_id
              FROM appointment
              WHERE status = 'vitals_taken' AND room_id IS NOT NULL
          )
        LIMIT 1
        "#
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| format!("Failed checking available clinic space: {}", e))?;

    if let Some(room) = free_room {
        sqlx::query!(
            r#"
            UPDATE appointment
            SET status = 'vitals_taken', room_id = $1, priority_level = $2, updated_at = NOW()
            WHERE id = $3
            "#,
            room.id,
            priority_level,
            appointment_id
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to assign room and update status: {}", e))?;
    } else {
        sqlx::query!(
            "UPDATE appointment SET status = 'vitals_taken', priority_level = $1, updated_at = NOW() WHERE id = $2",
            priority_level,
            appointment_id
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to update status: {}", e))?;
    }

    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(())
}

// Active prescriptions from the last 3 days for nurse medication administration
pub async fn get_active_prescriptions_for_nurse(
    pool: &PgPool,
) -> Result<Vec<serde_json::Value>, String> {
    let rows = sqlx::query!(
        r#"
        SELECT
            rx.id, rx.medicine_name, rx.dosage, rx.frequency,
            rx.duration, rx.instructions, rx.created_at,
            a.date AS appointment_date,
            p.first_name || ' ' || p.last_name AS patient_name,
            s.first_name || ' ' || s.last_name AS doctor_name,
            COUNT(mal.id)::INT AS admin_count
        FROM prescription rx
        JOIN appointment a ON rx.appointment_id = a.id
        JOIN patient     p ON a.patient_id      = p.id
        JOIN staff       s ON rx.prescribed_by_doctor_id = s.id
        LEFT JOIN medication_administration_log mal ON mal.prescription_id = rx.id
        WHERE rx.created_at >= NOW() - INTERVAL '3 days'
          AND a.status = 'admitted'
        GROUP BY rx.id, rx.medicine_name, rx.dosage, rx.frequency, rx.duration,
                 rx.instructions, rx.created_at, a.date,
                 p.first_name, p.last_name, s.first_name, s.last_name
        ORDER BY rx.created_at DESC
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id":            r.id,
                "medicine_name": r.medicine_name,
                "dosage":        r.dosage,
                "frequency":     r.frequency,
                "duration":      r.duration,
                "instructions":  r.instructions,
                "appt_date":     r.appointment_date.to_string(),
                "patient_name":  r.patient_name,
                "doctor_name":   r.doctor_name,
                "admin_count":   r.admin_count.unwrap_or(0),
            })
        })
        .collect())
}

// Log when a nurse administered a prescription dose
pub async fn log_medication_administration(
    pool: &PgPool,
    prescription_id: Uuid,
    nurse_id: Uuid,
    remarks: Option<String>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO medication_administration_log
            (prescription_id, administered_by_nurse_id, remarks)
        VALUES ($1, $2, $3)
        "#,
        prescription_id,
        nurse_id,
        remarks.as_deref(),
    )
    .execute(pool)
    .await?;
    Ok(())
}
