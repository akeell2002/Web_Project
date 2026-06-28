use sqlx::{PgPool, Row};
use uuid::Uuid;
use serde_json::Value;


// All rooms with computed occupancy status and current patient
pub async fn get_bed_overview(pool: &PgPool) -> Result<Vec<Value>, String> {
    let rows = sqlx::query(
        r#"
        SELECT
            r.id::text                                          AS room_id,
            r.room_name,
            r.room_type,
            r.location,
            r.bed_status,
            CASE
                WHEN r.bed_status = 'maintenance'              THEN 'maintenance'
                WHEN a.id IS NOT NULL                          THEN 'occupied'
                ELSE 'available'
            END                                                AS computed_status,
            p.first_name || ' ' || p.last_name                AS patient_name,
            p.id::text                                         AS patient_id,
            a.id::text                                         AS appointment_id,
            a.status::text                                     AS appointment_status
        FROM room r
        LEFT JOIN appointment a
            ON  a.room_id = r.id
            AND (
                  (a.date = CURRENT_DATE AND a.status IN ('checked_in', 'vitals_taken'))
                  OR a.status = 'admitted'
                )
        LEFT JOIN patient p ON a.patient_id = p.id
        ORDER BY r.room_type, r.room_name
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("get_bed_overview: {}", e))?;

    let beds = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "room_id":            r.get::<Option<String>, _>("room_id"),
                "room_name":          r.get::<String, _>("room_name"),
                "room_type":          r.get::<String, _>("room_type"),
                "location":           r.get::<String, _>("location"),
                "bed_status":         r.get::<String, _>("bed_status"),
                "computed_status":    r.get::<Option<String>, _>("computed_status").unwrap_or_else(|| "available".into()),
                "patient_name":       r.get::<Option<String>, _>("patient_name"),
                "patient_id":         r.get::<Option<String>, _>("patient_id"),
                "appointment_id":     r.get::<Option<String>, _>("appointment_id"),
                "appointment_status": r.get::<Option<String>, _>("appointment_status"),
            })
        })
        .collect();

    Ok(beds)
}

// Bed stats showing total beds, available, occupied, and under maintenance
pub async fn get_bed_stats(pool: &PgPool) -> Result<Value, String> {
    let row = sqlx::query(
        r#"
        SELECT
            COUNT(*)                                                           AS total_beds,
            COUNT(*) FILTER (WHERE r.bed_status != 'maintenance' AND a.id IS NULL) AS available,
            COUNT(*) FILTER (WHERE a.id IS NOT NULL)                           AS occupied,
            COUNT(*) FILTER (WHERE r.bed_status = 'maintenance')               AS maintenance
        FROM room r
        LEFT JOIN appointment a
            ON  a.room_id = r.id
            AND (
                  (a.date = CURRENT_DATE AND a.status IN ('checked_in', 'vitals_taken'))
                  OR a.status = 'admitted'
                )
        "#,
    )
    .fetch_one(pool)
    .await
    .map_err(|e| format!("get_bed_stats: {}", e))?;

    Ok(serde_json::json!({
        "total_beds":  row.get::<i64, _>("total_beds"),
        "available":   row.get::<i64, _>("available"),
        "occupied":    row.get::<i64, _>("occupied"),
        "maintenance": row.get::<i64, _>("maintenance"),
    }))
}

// All patients active in the clinic today - checked-in, vitals taken, or completed
pub async fn get_patient_census(pool: &PgPool) -> Result<Vec<Value>, String> {
    let rows = sqlx::query(
        r#"
        SELECT
            p.id::text                                              AS patient_id,
            p.first_name || ' ' || p.last_name                     AS patient_name,
            EXTRACT(YEAR FROM AGE(p.date_of_birth))::INT           AS age,
            COALESCE(p.gender, 'Unknown')                          AS gender,
            a.id::text                                             AS appointment_id,
            a.status::text                                         AS status,
            r.id::text                                             AS room_id,
            COALESCE(r.room_name, '-')                             AS room_name,
            COALESCE(r.room_type, '')                              AS room_type,
            COALESCE(s.first_name || ' ' || s.last_name, 'Unassigned') AS doctor_name,
            a.priority_level,
            COALESCE(mr.diagnosis, 'Pending Assessment')           AS condition
        FROM appointment a
        JOIN patient p ON a.patient_id = p.id
        LEFT JOIN room r         ON a.room_id   = r.id
        LEFT JOIN staff s        ON a.doctor_id = s.id
        LEFT JOIN medical_records mr ON mr.appointment_id = a.id
        WHERE (a.date = CURRENT_DATE AND a.status IN ('checked_in', 'vitals_taken', 'completed'))
           OR a.status = 'admitted'
        ORDER BY a.priority_level ASC NULLS LAST, a.queue_number ASC NULLS LAST
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("get_patient_census: {}", e))?;

    let patients = rows
        .iter()
        .map(|r| {
            let priority: i32 = r.get("priority_level");
            let status: String = r.get("status");

            let status_label = match status.as_str() {
                "checked_in"   => "Waiting",
                "vitals_taken" => "Vitals Taken",
                "admitted"     => "Admitted",
                "completed"    => "Discharged",
                _              => "Scheduled",
            };

            let priority_label = match priority {
                1 => "Critical",
                2 => "Severe",
                3 => "Moderate",
                4 => "Minor",
                _ => "Routine",
            };

            serde_json::json!({
                "patient_id":     r.get::<Option<String>, _>("patient_id"),
                "patient_name":   r.get::<String, _>("patient_name"),
                "age":            r.get::<Option<i32>, _>("age"),
                "gender":         r.get::<String, _>("gender"),
                "appointment_id": r.get::<Option<String>, _>("appointment_id"),
                "status":         status,
                "status_label":   status_label,
                "priority_level": priority,
                "priority_label": priority_label,
                "room_id":        r.get::<Option<String>, _>("room_id"),
                "room_name":      r.get::<String, _>("room_name"),
                "room_type":      r.get::<String, _>("room_type"),
                "doctor_name":    r.get::<String, _>("doctor_name"),
                "condition":      r.get::<String, _>("condition"),
            })
        })
        .collect();

    Ok(patients)
}

// Patient stats showing total patients, emergency, vitals taken, and discharged today
pub async fn get_patient_stats(pool: &PgPool) -> Result<Value, String> {
    let row = sqlx::query(
        r#"
        SELECT
            COUNT(DISTINCT a.patient_id)                                                 AS total_patients,
            COUNT(DISTINCT a.patient_id) FILTER (WHERE a.priority_level = 1)             AS emergency,
            COUNT(DISTINCT a.patient_id) FILTER (WHERE a.status = 'vitals_taken')        AS vitals_taken,
            COUNT(DISTINCT a.patient_id) FILTER (WHERE a.status = 'completed')           AS discharged_today
        FROM appointment a
        WHERE a.date   = CURRENT_DATE
          AND a.status IN ('checked_in', 'vitals_taken', 'completed')
        "#,
    )
    .fetch_one(pool)
    .await
    .map_err(|e| format!("get_patient_stats: {}", e))?;

    Ok(serde_json::json!({
        "total_patients":  row.get::<i64, _>("total_patients"),
        "emergency":       row.get::<i64, _>("emergency"),
        "vitals_taken":    row.get::<i64, _>("vitals_taken"),
        "discharged_today":row.get::<i64, _>("discharged_today"),
    }))
}

// All pending or approved bed transfer requests with patient and room details
pub async fn get_transfer_requests(pool: &PgPool) -> Result<Vec<Value>, String> {
    let rows = sqlx::query(
        r#"
        SELECT
            bt.id::text                                             AS transfer_id,
            bt.status,
            bt.reason,
            bt.created_at,
            p.first_name  || ' ' || p.last_name                   AS patient_name,
            p.id::text                                             AS patient_id,
            fr.room_name                                           AS from_room,
            tr.room_name                                           AS to_room,
            req.first_name || ' ' || req.last_name                AS requested_by,
            appr.first_name || ' ' || appr.last_name              AS approved_by
        FROM bed_transfers bt
        JOIN    patient p   ON bt.patient_id      = p.id
        LEFT JOIN room  fr  ON bt.from_room_id    = fr.id
        JOIN    room    tr  ON bt.to_room_id      = tr.id
        JOIN    staff   req ON bt.requested_by_id = req.id
        LEFT JOIN staff appr ON bt.approved_by_id = appr.id
        ORDER BY
            CASE bt.status WHEN 'pending' THEN 0 WHEN 'approved' THEN 1 ELSE 2 END,
            bt.created_at DESC
        LIMIT 30
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("get_transfer_requests: {}", e))?;

    let transfers = rows
        .iter()
        .map(|r| {
            let created_at: chrono::DateTime<chrono::Utc> = r.get("created_at");
            let mins_ago = (chrono::Utc::now() - created_at).num_minutes();
            let time_ago = if mins_ago < 60 {
                format!("{}m ago", mins_ago)
            } else {
                format!("{}h ago", mins_ago / 60)
            };

            serde_json::json!({
                "transfer_id":   r.get::<Option<String>, _>("transfer_id"),
                "status":        r.get::<String, _>("status"),
                "reason":        r.get::<Option<String>, _>("reason"),
                "patient_name":  r.get::<String, _>("patient_name"),
                "patient_id":    r.get::<Option<String>, _>("patient_id"),
                "from_room":     r.get::<Option<String>, _>("from_room").unwrap_or_else(|| "-".into()),
                "to_room":       r.get::<String, _>("to_room"),
                "requested_by":  r.get::<String, _>("requested_by"),
                "approved_by":   r.get::<Option<String>, _>("approved_by"),
                "time_ago":      time_ago,
            })
        })
        .collect();

    Ok(transfers)
}

// Create a new bed transfer request by doctor or nurse
pub async fn create_transfer_request(
    pool: &PgPool,
    patient_id: Uuid,
    from_room_id: Option<Uuid>,
    to_room_id: Uuid,
    requested_by_id: Uuid,
    reason: Option<String>,
) -> Result<Uuid, String> {
    let row = sqlx::query(
        r#"
        INSERT INTO bed_transfers (patient_id, from_room_id, to_room_id, requested_by_id, reason)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id
        "#,
    )
    .bind(patient_id)
    .bind(from_room_id)
    .bind(to_room_id)
    .bind(requested_by_id)
    .bind(reason)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("create_transfer_request: {}", e))?;

    Ok(row.get("id"))
}

// Doctor to approve a pending bed transfer request
pub async fn approve_transfer(
    pool: &PgPool,
    transfer_id: Uuid,
    approved_by_id: Uuid,
) -> Result<(), String> {
    // Fetch transfer details first
    let row = sqlx::query(
        "SELECT patient_id, to_room_id FROM bed_transfers WHERE id = $1 AND status = 'pending'",
    )
    .bind(transfer_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("approve_transfer fetch: {}", e))?
    .ok_or_else(|| "Transfer not found or already actioned".to_string())?;

    let patient_id: Uuid = row.get("patient_id");
    let to_room_id: Uuid = row.get("to_room_id");

    let mut tx = pool.begin().await.map_err(|e| format!("tx begin: {}", e))?;

    // Move the patient's active appointment to the new room
    sqlx::query(
        r#"
        UPDATE appointment
        SET    room_id     = $1,
               updated_at  = NOW()
        WHERE  patient_id  = $2
          AND  (
                 (date = CURRENT_DATE AND status IN ('checked_in', 'vitals_taken'))
                 OR status = 'admitted'
               )
        "#,
    )
    .bind(to_room_id)
    .bind(patient_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("approve_transfer update appt: {}", e))?;

    // Mark the transfer approved
    sqlx::query(
        r#"
        UPDATE bed_transfers
        SET    status         = 'approved',
               approved_by_id = $1,
               updated_at     = NOW()
        WHERE  id             = $2
        "#,
    )
    .bind(approved_by_id)
    .bind(transfer_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("approve_transfer update transfer: {}", e))?;

    tx.commit().await.map_err(|e| format!("approve_transfer commit: {}", e))?;
    Ok(())
}

// Doctor to reject a pending bed transfer request
pub async fn reject_transfer(
    pool: &PgPool,
    transfer_id: Uuid,
    rejected_by_id: Uuid,
) -> Result<(), String> {
    sqlx::query(
        r#"
        UPDATE bed_transfers
        SET    status         = 'rejected',
               approved_by_id = $1,
               updated_at     = NOW()
        WHERE  id             = $2
          AND  status         = 'pending'
        "#,
    )
    .bind(rejected_by_id)
    .bind(transfer_id)
    .execute(pool)
    .await
    .map_err(|e| format!("reject_transfer: {}", e))?;

    Ok(())
}

// Update the bed status of a room
pub async fn set_room_status(pool: &PgPool, room_id: Uuid, status: &str) -> Result<(), String> {
    sqlx::query("UPDATE room SET bed_status = $1 WHERE id = $2")
        .bind(status)
        .bind(room_id)
        .execute(pool)
        .await
        .map_err(|e| format!("set_room_status: {}", e))?;
    Ok(())
}

// Discharge a patient and generate the final bill, close the bill and free the bed
pub async fn discharge_patient(pool: &PgPool, appointment_id: Uuid) -> Result<(), String> {
    let mut tx = pool.begin().await.map_err(|e| format!("discharge begin: {}", e))?;

    // Fetch the appointment details
    let appt = sqlx::query(
        r#"
        SELECT patient_id,
               priority_level,
               GREATEST(1, (CURRENT_DATE - date))::INT AS nights
        FROM appointment
        WHERE id = $1 AND status = 'admitted'::appointment_status
        "#,
    )
    .bind(appointment_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| format!("discharge fetch appt: {}", e))?;

    let appt = match appt {
        Some(r) => r,
        None => return Err("Patient is not currently admitted.".to_string()),
    };

    let patient_id:     Uuid = appt.get("patient_id");
    let priority_level: i32  = appt.get("priority_level");
    let nights:         i32  = appt.get("nights");

    // Sum medicine charges across every prescription written during the visit
    let med_rows = sqlx::query("SELECT medicine_name FROM prescription WHERE appointment_id = $1")
        .bind(appointment_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| format!("discharge fetch meds: {}", e))?;

    let medicine_fee: f64 = med_rows
        .iter()
        .map(|r| crate::pricing::medicine_price(&r.get::<String, _>("medicine_name")))
        .sum();

    let consultation_fee = crate::pricing::consultation_fee(priority_level);
    let admission_fee    = crate::pricing::admission_fee(nights as i64);
    let total            = consultation_fee + medicine_fee + admission_fee;

    // Generate the discharge bill
    sqlx::query(
        r#"
        INSERT INTO bills
            (patient_id, appointment_id, consultation_fee, medicine_fee, admission_fee, total_amount, payment_status)
        VALUES ($1, $2, $3::numeric, $4::numeric, $5::numeric, $6::numeric, 'unpaid'::bill_status)
        "#,
    )
    .bind(patient_id)
    .bind(appointment_id)
    .bind(consultation_fee)
    .bind(medicine_fee)
    .bind(admission_fee)
    .bind(total)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("discharge insert bill: {}", e))?;

    // Close the case and free the bed
    sqlx::query(
        r#"
        UPDATE appointment
        SET    status     = 'completed'::appointment_status,
               room_id    = NULL,
               updated_at = NOW()
        WHERE  id = $1
        "#,
    )
    .bind(appointment_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("discharge update appt: {}", e))?;

    tx.commit().await.map_err(|e| format!("discharge commit: {}", e))?;
    Ok(())
}