use sqlx::PgPool;
use uuid::Uuid;
use chrono::{NaiveDate, NaiveTime, Duration};
use crate::models::appointment::Appointment;

/// Fetches all active start and end times for a specific doctor on a given day
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

/// Fetches all active start and end times for a specific patient on a given day
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

/// Safely inserts a new appointment into the database after final backend overlap validation
pub async fn book_patient_appointment(
    pool: &sqlx::PgPool,
    patient_id: uuid::Uuid,
    doctor_id: uuid::Uuid,
    date: chrono::NaiveDate,
    start_time: chrono::NaiveTime,
    end_time: chrono::NaiveTime, // Added 6th argument to handle variable durations
) -> Result<crate::models::appointment::Appointment, String> {
    
    // Double-check overlap boundaries right before database insertion to ensure structural integrity
    let conflict = sqlx::query!(
        r#"
        SELECT 
            EXISTS (
                SELECT 1 FROM appointment
                WHERE doctor_id = $1 AND date = $2 AND status NOT IN ('cancelled', 'no_show') AND (start_time < $4 AND end_time > $3)
            ) as "doctor_conflict!",
            EXISTS (
                SELECT 1 FROM appointment
                WHERE patient_id = $5 AND date = $2 AND status NOT IN ('cancelled', 'no_show') AND (start_time < $4 AND end_time > $3)
            ) as "patient_conflict!"
        "#,
        doctor_id,
        date,
        start_time,
        end_time,
        patient_id
    )
    .fetch_one(pool)
    .await
    .map_err(|e| format!("Database verification failed: {}", e))?;

    if conflict.doctor_conflict {
        return Err("This doctor is already booked for an appointment during this time slot.".to_string());
    }
    if conflict.patient_conflict {
        return Err("You already have an active appointment booked during this time window.".to_string());
    }

    let appointment = sqlx::query_as!(
        crate::models::appointment::Appointment,
        r#"
        INSERT INTO appointment (patient_id, doctor_id, room_id, date, start_time, end_time, created_by)
        VALUES ($1, $2, NULL, $3, $4, $5, $1)
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
        end_time
    )
    .fetch_one(pool)
    .await
    .map_err(|e| format!("Failed to insert appointment row: {}", e))?;

    Ok(appointment)
}

/// Retrieve all appointments for a specific patient, enriched with doctor details
/// Retrieve all appointments for a specific patient, enriched with doctor details
pub async fn get_patient_appointments(
    pool: &sqlx::PgPool,
    patient_id: uuid::Uuid,
) -> Result<Vec<serde_json::Value>, String> {
    let rows = sqlx::query!(
        r#"
        SELECT 
            a.id,
            a.date,
            a.start_time,
            a.end_time,
            a.status::text as "status!",
            s.first_name as "doc_first?",
            s.last_name as "doc_last?"
        FROM appointment a
        LEFT JOIN staff s ON a.doctor_id = s.id
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

    let mut list = Vec::new();
    for row in rows {
        let doc_name = match (row.doc_first, row.doc_last) {
            (Some(f), Some(l)) => format!("Dr. {} {}", f, l),
            _ => "Assigned Practitioner".to_string(),
        };

        // Classify timeline status context based on real current system clock parameters
        let is_upcoming = if row.date > now_date {
            true
        } else if row.date == now_date {
            row.start_time >= now_time
        } else {
            false
        };

        list.push(serde_json::json!({
            "id": row.id,
            "date": row.date.to_string(),
            "formatted_date": row.date.format("%A, %b %d, %Y").to_string(),
            "start_time": row.start_time.format("%I:%M %p").to_string(),
            "end_time": row.end_time.format("%I:%M %p").to_string(),
            "status": row.status,
            "doctor_name": doc_name,
            "is_upcoming": is_upcoming
        }));
    }

    Ok(list)
}

/// Retrieves appointments assigned to a specific practitioner from today onwards,
/// with dynamic filtering options synchronized with the application clock.
pub async fn get_doctor_daily_appointments(
    pool: &sqlx::PgPool,
    doctor_id: uuid::Uuid,
    filter_mode: &str,
) -> Result<Vec<serde_json::Value>, String> {
    
    let mut list = Vec::new();
    let today_date = chrono::Local::now().date_naive();

    if filter_mode == "today" {
        // CHANGED: Swapped CURRENT_DATE out for an explicit bind variable ($2)
        let rows = sqlx::query!(
            r#"
            SELECT a.id, a.date, a.start_time, a.end_time, a.status::text as "status!", a.queue_number,
                   p.first_name, p.last_name, p.date_of_birth, p.gender
            FROM appointment a
            JOIN patient p ON a.patient_id = p.id
            WHERE a.doctor_id = $1 AND a.date = $2
            ORDER BY 
                CASE WHEN a.status = 'checked_in' THEN 1 WHEN a.status = 'scheduled' THEN 2 ELSE 3 END ASC,
                a.queue_number ASC, a.start_time ASC
            "#,
            doctor_id,
            today_date
        )
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Failed to query daily clinical queue: {}", e))?;

        for row in rows {
            list.push(serde_json::json!({
                "id": row.id,
                "appointment_date": row.date.format("%A, %b %d, %Y").to_string(),
                "is_today": true,
                "start_time": row.start_time.format("%I:%M %p").to_string(),
                "end_time": row.end_time.format("%I:%M %p").to_string(),
                "status": row.status,
                "queue_number": row.queue_number,
                "patient_name": format!("{} {}", row.first_name, row.last_name),
                "patient_dob": row.date_of_birth.to_string(),
                "patient_gender": row.gender.unwrap_or_else(|| "Undisclosed".to_string())
            }));
        }
    } else {
        // CHANGED: Swapped CURRENT_DATE out for an explicit bind variable ($2)
        let rows = sqlx::query!(
            r#"
            SELECT a.id, a.date, a.start_time, a.end_time, a.status::text as "status!", a.queue_number,
                   p.first_name, p.last_name, p.date_of_birth, p.gender
            FROM appointment a
            JOIN patient p ON a.patient_id = p.id
            WHERE a.doctor_id = $1 AND a.date >= $2
            ORDER BY a.date ASC, a.start_time ASC
            "#,
            doctor_id,
            today_date
        )
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Failed to query upcoming clinical caseload: {}", e))?;

        for row in rows {
            list.push(serde_json::json!({
                "id": row.id,
                "appointment_date": row.date.format("%A, %b %d, %Y").to_string(),
                "is_today": row.date == today_date,
                "start_time": row.start_time.format("%I:%M %p").to_string(),
                "end_time": row.end_time.format("%I:%M %p").to_string(),
                "status": row.status,
                "queue_number": row.queue_number,
                "patient_name": format!("{} {}", row.first_name, row.last_name),
                "patient_dob": row.date_of_birth.to_string(),
                "patient_gender": row.gender.unwrap_or_else(|| "Undisclosed".to_string())
            }));
        }
    }

    Ok(list)
}