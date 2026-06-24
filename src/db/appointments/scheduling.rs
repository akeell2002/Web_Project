use sqlx::PgPool;
use uuid::Uuid;
use chrono::{NaiveDate, NaiveTime};

/// Fetches all active busy periods for a specific doctor on a given day
pub async fn get_doctor_busy_periods(
    pool: &PgPool,
    doctor_id: Uuid,
    date: NaiveDate,
) -> Result<Vec<(NaiveTime, NaiveTime)>, String> {
    let rows = sqlx::query!(
        r#"
        SELECT start_time, end_time
        FROM appointment
        WHERE doctor_id = $1
          AND date = $2
          AND status NOT IN ('cancelled', 'no_show')
        "#,
        doctor_id,
        date
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to fetch doctor busy periods: {}", e))?;

    Ok(rows.into_iter().map(|r| (r.start_time, r.end_time)).collect())
}

/// Fetches all active busy periods for a specific patient on a given day
pub async fn get_patient_busy_periods(
    pool: &PgPool,
    patient_id: Uuid,
    date: NaiveDate,
) -> Result<Vec<(NaiveTime, NaiveTime)>, String> {
    let rows = sqlx::query!(
        r#"
        SELECT start_time, end_time
        FROM appointment
        WHERE patient_id = $1
          AND date = $2
          AND status NOT IN ('cancelled', 'no_show')
        "#,
        patient_id,
        date
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to fetch patient busy periods: {}", e))?;

    Ok(rows.into_iter().map(|r| (r.start_time, r.end_time)).collect())
}

/// Atomic INSERT that prevents double-booking for both the doctor and the patient
pub async fn book_patient_appointment(
    pool: &PgPool,
    patient_id: Uuid,
    doctor_id: Uuid,
    date: NaiveDate,
    start_time: NaiveTime,
    end_time: NaiveTime,
    priority_level: i32, 
) -> Result<crate::models::appointment::Appointment, String> {
    let appointment = sqlx::query_as!(
        crate::models::appointment::Appointment,
        r#"
        /* ADD priority_level to the columns, and $6 to the SELECT clause */
        INSERT INTO appointment (patient_id, doctor_id, room_id, date, start_time, end_time, created_by, priority_level)
        SELECT $1, $2, NULL, $3, $4, $5, $1, $6
        WHERE NOT EXISTS (
            SELECT 1 FROM appointment
            WHERE doctor_id = $2 AND date = $3
              AND status NOT IN ('cancelled', 'no_show')
              AND (start_time < $5 AND end_time > $4)
        ) AND NOT EXISTS (
            SELECT 1 FROM appointment
            WHERE patient_id = $1 AND date = $3
              AND status NOT IN ('cancelled', 'no_show')
              AND (start_time < $5 AND end_time > $4)
        )
        RETURNING
            id, patient_id, doctor_id, room_id,
            status::text as "status!",
            date, start_time, end_time, queue_number, check_in_time,
            created_by, created_at, updated_at
        "#,
        patient_id,
        doctor_id,
        date,
        start_time,
        end_time,
        priority_level
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Failed to execute booking query: {}", e))?;

    match appointment {
        Some(appt) => Ok(appt),
        None => Err("Booking failed: This time slot was just taken or you already have a conflicting appointment.".to_string()),
    }
}

/// All appointments for a specific patient, enriched with doctor and room info
pub async fn get_patient_appointments(
    pool: &PgPool,
    patient_id: Uuid,
) -> Result<Vec<serde_json::Value>, String> {
    let rows = sqlx::query!(
        r#"
        SELECT
            a.id, a.date, a.start_time, a.end_time,
            a.status::text as "status!",
            s.first_name as "doc_first?",
            s.last_name  as "doc_last?",
            r.room_name  as "room_name?"
        FROM appointment a
        LEFT JOIN staff s ON a.doctor_id = s.id
        LEFT JOIN room  r ON a.room_id   = r.id
        WHERE a.patient_id = $1
        ORDER BY a.date DESC, a.start_time DESC
        "#,
        patient_id
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to fetch patient appointments: {}", e))?;

    let now_date = chrono::Local::now().date_naive();
    let now_time = chrono::Local::now().time();

    let list = rows
        .into_iter()
        .map(|row| {
            let doc_name = match (row.doc_first, row.doc_last) {
                (Some(f), Some(l)) => format!("Dr. {} {}", f, l),
                _ => "Assigned Practitioner".to_string(),
            };
            let is_terminal = matches!(row.status.as_str(), "cancelled" | "no_show");
            let is_upcoming = !is_terminal && if row.date > now_date {
                true
            } else if row.date == now_date {
                row.start_time >= now_time
            } else {
                false
            };
            serde_json::json!({
                "id":             row.id,
                "date":           row.date.to_string(),
                "formatted_date": row.date.format("%A, %b %d, %Y").to_string(),
                "start_time":     row.start_time.format("%I:%M %p").to_string(),
                "end_time":       row.end_time.format("%I:%M %p").to_string(),
                "status":         row.status,
                "doctor_name":    doc_name,
                "is_upcoming":    is_upcoming,
                "room":           row.room_name.unwrap_or_else(|| "Waiting Area".to_string()),
            })
        })
        .collect();

    Ok(list)
}

/// Full clinic schedule for today - used by the receptionist
pub async fn get_today_clinic_schedule(pool: &PgPool) -> Result<Vec<serde_json::Value>, String> {
    let today = chrono::Local::now().date_naive();

    let rows = sqlx::query!(
        r#"
        SELECT a.id, a.start_time, a.status::text as "status!", a.queue_number,
               a.priority_level,
               p.first_name as patient_first, p.last_name as patient_last,
               s.first_name as doc_first,     s.last_name as doc_last,
               r.room_name  as "room_name?"
        FROM appointment a
        JOIN patient p ON a.patient_id = p.id
        JOIN staff   s ON a.doctor_id  = s.id
        LEFT JOIN room r ON a.room_id  = r.id
        WHERE a.date = $1
        ORDER BY a.start_time ASC
        "#,
        today
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to fetch clinic schedule: {}", e))?;

    Ok(rows
        .into_iter()
        .map(|row| {
            serde_json::json!({
                "id":           row.id,
                "time":         row.start_time.format("%I:%M %p").to_string(),
                "status":       row.status,
                "queue_number": row.queue_number,
                "priority":     row.priority_level,
                "patient_name": format!("{} {}", row.patient_first, row.patient_last),
                "doctor_name":  format!("Dr. {}", row.doc_last),
                "room":         row.room_name.unwrap_or_else(|| "Unassigned".to_string()),
            })
        })
        .collect())
}

/// Check a patient in and assign the next queue number safely using transaction locks
pub async fn check_in_patient(pool: &PgPool, appointment_id: Uuid) -> Result<i32, String> {
    // 1. Open an atomic transaction
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    // 2. Look up the doctor associated with this appointment
    let appt_meta = sqlx::query!(
        "SELECT doctor_id FROM appointment WHERE id = $1", 
        appointment_id
    ).fetch_one(&mut *tx).await.map_err(|e| e.to_string())?;

    // 3. Apply a Postgres Advisory Lock mapped to the doctor's ID.
    // This forces competing check-ins for the SAME doctor to wait in a single-file line,
    // without blocking check-ins for OTHER doctors.
    let lock_id = (appt_meta.doctor_id.unwrap_or_default().as_u128() % 2147483647) as i64;
    sqlx::query!("SELECT pg_advisory_xact_lock($1)", lock_id)
        .execute(&mut *tx).await.map_err(|e| e.to_string())?;

    // 4. Now it is mathematically safe to SELECT MAX and UPDATE
    let record = sqlx::query!(
        r#"
        UPDATE appointment
        SET status       = 'checked_in',
            check_in_time = CURRENT_TIMESTAMP,
            queue_number  = (
                SELECT COALESCE(MAX(queue_number), 0) + 1
                FROM appointment
                WHERE doctor_id = appointment.doctor_id AND date = appointment.date
            )
        WHERE id = $1 AND status = 'scheduled'
        RETURNING queue_number
        "#,
        appointment_id
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| format!("Failed to update check-in status: {}", e))?;

    // 5. Commit the transaction (which automatically releases the lock)
    tx.commit().await.map_err(|e| e.to_string())?;

    match record {
        Some(row) => Ok(row.queue_number.unwrap_or(0)),
        None => Err("Check-in failed: Appointment not found or already checked in.".to_string()),
    }
}

/// Mark an appointment as no_show (receptionist action, appointment must be scheduled/checked_in)
pub async fn mark_appointment_no_show(
    pool:           &PgPool,
    appointment_id: Uuid,
) -> Result<(), String> {
    sqlx::query!(
        r#"
        UPDATE appointment
        SET status     = 'no_show'::appointment_status,
            updated_at = NOW()
        WHERE id     = $1
          AND status IN ('scheduled'::appointment_status, 'checked_in'::appointment_status)
        "#,
        appointment_id
    )
    .execute(pool)
    .await
    .map_err(|e| format!("Failed to mark no_show: {}", e))?;
    Ok(())
}

/// Fetch a single appointment by ID for a specific patient (ownership check)
pub async fn get_patient_appointment_by_id(
    pool:           &PgPool,
    appointment_id: Uuid,
    patient_id:     Uuid,
) -> Result<Option<crate::models::appointment::Appointment>, sqlx::Error> {
    sqlx::query_as!(
        crate::models::appointment::Appointment,
        r#"
        SELECT id, patient_id, doctor_id, room_id,
               status::text as "status!",
               date, start_time, end_time, queue_number,
               check_in_time, created_by as "created_by?",
               created_at, updated_at
        FROM appointment
        WHERE id = $1 AND patient_id = $2
        "#,
        appointment_id,
        patient_id
    )
    .fetch_optional(pool)
    .await
}

/// Update an existing patient appointment (reschedule)
pub async fn update_patient_appointment(
    pool:           &PgPool,
    appointment_id: Uuid,
    patient_id:     Uuid,
    doctor_id:      Uuid,
    date:           chrono::NaiveDate,
    start_time:     chrono::NaiveTime,
    end_time:       chrono::NaiveTime,
    priority:       i32,
) -> Result<(), sqlx::Error> {
    let conflict = sqlx::query!(
        r#"
        SELECT id FROM appointment
        WHERE doctor_id = $1
          AND date = $2
          AND id != $3
          AND status NOT IN ('cancelled', 'no_show')
          AND start_time < $5
          AND end_time   > $4
        LIMIT 1
        "#,
        doctor_id, date, appointment_id, start_time, end_time
    )
    .fetch_optional(pool)
    .await?;

    if conflict.is_some() {
        return Err(sqlx::Error::Protocol("Time slot is already booked.".into()));
    }

    sqlx::query!(
        r#"
        UPDATE appointment
        SET doctor_id      = $1,
            date          = $2,
            start_time    = $3,
            end_time      = $4,
            priority_level = $5,
            updated_at    = NOW()
        WHERE id = $6 AND patient_id = $7
        "#,
        doctor_id, date, start_time, end_time, priority, appointment_id, patient_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Cancel a patient's own appointment (only if still scheduled)
pub async fn cancel_patient_appointment(
    pool:           &PgPool,
    appointment_id: Uuid,
    patient_id:     Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        UPDATE appointment
        SET status     = 'cancelled'::appointment_status,
            updated_at = NOW()
        WHERE id = $1 AND patient_id = $2
          AND status = 'scheduled'::appointment_status
        "#,
        appointment_id,
        patient_id
    )
    .execute(pool)
    .await?;
    Ok(())
}
