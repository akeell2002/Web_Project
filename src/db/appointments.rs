use sqlx::PgPool;
use uuid::Uuid;
use chrono::{NaiveDate, NaiveTime};

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

/// Safely inserts a new appointment into the database using an atomic INSERT to prevent race conditions.
pub async fn book_patient_appointment(
    pool: &sqlx::PgPool,
    patient_id: uuid::Uuid,
    doctor_id: uuid::Uuid,
    date: chrono::NaiveDate,
    start_time: chrono::NaiveTime,
    end_time: chrono::NaiveTime, 
) -> Result<crate::models::appointment::Appointment, String> {
    
    // Atomic INSERT: It will only insert if BOTH the doctor and patient are free.
    // If a conflict exists, it inserts nothing and returns 0 rows.
    let appointment = sqlx::query_as!(
        crate::models::appointment::Appointment,
        r#"
        INSERT INTO appointment (patient_id, doctor_id, room_id, date, start_time, end_time, created_by)
        SELECT $1, $2, NULL, $3, $4, $5, $1
        WHERE NOT EXISTS (
            SELECT 1 FROM appointment 
            WHERE doctor_id = $2 AND date = $3 AND status NOT IN ('cancelled', 'no_show') AND (start_time < $5 AND end_time > $4)
        ) AND NOT EXISTS (
            SELECT 1 FROM appointment 
            WHERE patient_id = $1 AND date = $3 AND status NOT IN ('cancelled', 'no_show') AND (start_time < $5 AND end_time > $4)
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
        end_time
    )
    .fetch_optional(pool) // Use fetch_optional because it might return None if blocked by the WHERE NOT EXISTS
    .await
    .map_err(|e| format!("Failed to execute booking query: {}", e))?;

    match appointment {
        Some(appt) => Ok(appt),
        None => Err("Booking failed: This time slot was just taken or you already have a conflicting appointment.".to_string()),
    }
}


// Retrieve all appointments for a specific patient, enriched with doctor details and room layout tracking
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
            s.last_name as "doc_last?",
            r.room_name as "room_name?"
        FROM appointment a
        LEFT JOIN staff s ON a.doctor_id = s.id
        LEFT JOIN room r ON a.room_id = r.id   
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
            "is_upcoming": is_upcoming,
            "room": row.room_name.unwrap_or_else(|| "Waiting Area".to_string()) 
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
        let rows = sqlx::query!(
            r#"
            SELECT a.id, a.date, a.start_time, a.end_time, a.status::text as "status!", a.queue_number,
                   p.first_name, p.last_name, p.date_of_birth, p.gender,
                   r.room_name as "room_name?",
                   tv.blood_pressure, 
                   tv.temperature::FLOAT8 as "temperature?", 
                   tv.weight_kg::FLOAT8 as "weight_kg?", 
                   tv.height_cm::FLOAT8 as "height_cm?"
            FROM appointment a
            JOIN patient p ON a.patient_id = p.id
            LEFT JOIN room r ON a.room_id = r.id
            LEFT JOIN triage_vitals tv ON a.id = tv.appointment_id
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
            let bp = row.blood_pressure.unwrap_or_else(|| "--".to_string());
            let temp = row.temperature.map(|t| format!("{:.1} °C", t)).unwrap_or_else(|| "--".to_string());
            let weight = row.weight_kg.map(|w| format!("{:.1} kg", w)).unwrap_or_else(|| "--".to_string());
            let height = row.height_cm.map(|h| format!("{:.1} cm", h)).unwrap_or_else(|| "--".to_string());
            let room_display = row.room_name.unwrap_or_else(|| "Waiting Area".to_string());

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
                "patient_gender": row.gender.unwrap_or_else(|| "Undisclosed".to_string()),
                "room": room_display,
                "blood_pressure": bp,
                "temperature": temp,
                "weight": weight,
                "height": height
            }));
        }
    } else {
        let rows = sqlx::query!(
            r#"
            SELECT a.id, a.date, a.start_time, a.end_time, a.status::text as "status!", a.queue_number,
                   p.first_name, p.last_name, p.date_of_birth, p.gender,
                   tv.blood_pressure, 
                   tv.temperature::FLOAT8 as "temperature?", 
                   tv.weight_kg::FLOAT8 as "weight_kg?", 
                   tv.height_cm::FLOAT8 as "height_cm?"
            FROM appointment a
            JOIN patient p ON a.patient_id = p.id
            LEFT JOIN triage_vitals tv ON a.id = tv.appointment_id
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
            let bp = row.blood_pressure.unwrap_or_else(|| "--".to_string());
            let temp = row.temperature.map(|t| format!("{:.1} °C", t)).unwrap_or_else(|| "--".to_string());
            let weight = row.weight_kg.map(|w| format!("{:.1} kg", w)).unwrap_or_else(|| "--".to_string());
            let height = row.height_cm.map(|h| format!("{:.1} cm", h)).unwrap_or_else(|| "--".to_string());

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
                "patient_gender": row.gender.unwrap_or_else(|| "Undisclosed".to_string()),
                "blood_pressure": bp,
                "temperature": temp,
                "weight": weight,
                "height": height
            }));
        }
    }

    Ok(list)
}


/// Fetches all appointments for TODAY across the whole clinic for the Receptionist
pub async fn get_today_clinic_schedule(
    pool: &sqlx::PgPool,
) -> Result<Vec<serde_json::Value>, String> {
    let today_date = chrono::Local::now().date_naive();

    let rows = sqlx::query!(
        r#"
        SELECT a.id, a.start_time, a.status::text as "status!", a.queue_number,
               p.first_name as patient_first, p.last_name as patient_last,
               s.first_name as doc_first, s.last_name as doc_last,
               r.room_name as "room_name?"
        FROM appointment a
        JOIN patient p ON a.patient_id = p.id
        JOIN staff s ON a.doctor_id = s.id
        LEFT JOIN room r ON a.room_id = r.id
        WHERE a.date = $1
        ORDER BY a.start_time ASC
        "#,
        today_date
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to fetch clinic schedule: {}", e))?;

    let mut list = Vec::new();
    for row in rows {
        list.push(serde_json::json!({
            "id": row.id,
            "time": row.start_time.format("%I:%M %p").to_string(),
            "status": row.status,
            "queue_number": row.queue_number,
            "patient_name": format!("{} {}", row.patient_first, row.patient_last),
            "doctor_name": format!("Dr. {}", row.doc_last),
            "room": row.room_name.unwrap_or_else(|| "Unassigned".to_string())
        }));
    }
    Ok(list)
}

/// Updates an appointment to 'checked_in' and assigns the next available queue number.
pub async fn check_in_patient(
    pool: &sqlx::PgPool,
    appointment_id: uuid::Uuid,
) -> Result<i32, String> {
    // We use COALESCE to handle the very first patient of the day (when MAX is null, it becomes 0, then we add 1).
    let record = sqlx::query!(
        r#"
        UPDATE appointment
        SET 
            status = 'checked_in',
            check_in_time = CURRENT_TIMESTAMP,
            queue_number = (
                SELECT COALESCE(MAX(queue_number), 0) + 1 
                FROM appointment 
                WHERE doctor_id = appointment.doctor_id AND date = appointment.date
            )
        WHERE id = $1 AND status = 'scheduled'
        RETURNING queue_number
        "#,
        appointment_id
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Failed to update check-in status: {}", e))?;

    match record {
        Some(row) => Ok(row.queue_number.unwrap_or(0)),
        None => Err("Check-in failed: Appointment not found or already checked in.".to_string()),
    }
}

// --- Moved from db/triage.rs ---

pub async fn get_triage_queue(pool: &PgPool) -> Result<Vec<serde_json::Value>, String> {
    let today_date = chrono::Local::now().date_naive();

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
        list.push(serde_json::json!({
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

// --- Moved from db/consultation.rs ---

pub async fn finalize_consultation_and_bill(
    pool: &PgPool,
    appointment_id: Uuid,
    form: crate::models::appointment::EncounterForm,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    let appointment = sqlx::query!(
        r#"
        SELECT patient_id, doctor_id
        FROM appointment
        WHERE id = $1
        "#,
        appointment_id
    )
    .fetch_one(&mut *tx)
    .await?;

    let patient_id = appointment.patient_id;
    let doctor_id = appointment.doctor_id.expect("A doctor must be assigned to the appointment.");

    sqlx::query!(
        r#"
        INSERT INTO medical_records (patient_id, appointment_id, doctor_id, symptoms, diagnosis, treatment_notes)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        patient_id,
        appointment_id,
        doctor_id,
        form.symptoms,
        form.diagnosis,
        form.treatment_notes
    )
    .execute(&mut *tx)
    .await?;

    let mut medicine_fee: f64 = 0.00;
    let consultation_fee: f64 = 50.00;

    if let Some(medicine) = form.medicine_name {
        if !medicine.trim().is_empty() {
            sqlx::query!(
                r#"
                INSERT INTO prescription (appointment_id, prescribed_by_doctor_id, medicine_name, dosage, frequency, duration, instructions)
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                "#,
                appointment_id,
                doctor_id,
                medicine,
                form.dosage.unwrap_or_default(),
                form.frequency.unwrap_or_default(),
                form.duration.unwrap_or_default(),
                form.instructions
            )
            .execute(&mut *tx)
            .await?;

            medicine_fee = 20.00;
        }
    }

    let total_amount = consultation_fee + medicine_fee;

    sqlx::query!(
        r#"
        INSERT INTO bills (patient_id, appointment_id, consultation_fee, medicine_fee, total_amount, payment_status)
        VALUES ($1, $2, $3::FLOAT8, $4::FLOAT8, $5::FLOAT8, 'unpaid')
        "#,
        patient_id,
        appointment_id,
        consultation_fee,
        medicine_fee,
        total_amount
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        r#"
        UPDATE appointment
        SET status = 'completed'::appointment_status,
            room_id = NULL,
            updated_at = NOW()
        WHERE id = $1
        "#,
        appointment_id
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}

/// Fetch active prescriptions for the medication administration log (nurse)
pub async fn get_active_prescriptions_for_nurse(
    pool: &PgPool,
) -> Result<Vec<serde_json::Value>, String> {
    let today_date = chrono::Local::now().date_naive();

    let rows = sqlx::query(
        r#"
        SELECT
            pr.id,
            pr.medicine_name,
            pr.dosage,
            pr.frequency,
            pr.duration,
            pr.instructions,
            pr.created_at,
            p.first_name AS patient_first,
            p.last_name  AS patient_last,
            s.first_name AS doctor_first,
            s.last_name  AS doctor_last,
            a.date       AS appt_date,
            (SELECT COUNT(*) FROM medication_administration_log mal WHERE mal.prescription_id = pr.id) AS admin_count
        FROM prescription pr
        JOIN appointment a ON pr.appointment_id = a.id
        JOIN patient p ON a.patient_id = p.id
        JOIN staff s ON pr.prescribed_by_doctor_id = s.id
        WHERE a.date >= ($1::date - INTERVAL '3 days')
        ORDER BY a.date DESC, pr.created_at DESC
        "#,
    )
    .bind(today_date)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("DB error fetching prescriptions for nurse: {}", e))?;

    use sqlx::Row;
    let mut list = Vec::new();
    for row in rows {
        let id: uuid::Uuid                  = row.get("id");
        let medicine: String                = row.get("medicine_name");
        let dosage: String                  = row.get("dosage");
        let frequency: String               = row.get("frequency");
        let duration: String                = row.get("duration");
        let instructions: Option<String>    = row.get("instructions");
        let patient_first: String           = row.get("patient_first");
        let patient_last: String            = row.get("patient_last");
        let doctor_first: String            = row.get("doctor_first");
        let doctor_last: String             = row.get("doctor_last");
        let appt_date: chrono::NaiveDate    = row.get("appt_date");
        let admin_count: i64                = row.get("admin_count");

        list.push(serde_json::json!({
            "id": id.to_string(),
            "medicine_name": medicine,
            "dosage": dosage,
            "frequency": frequency,
            "duration": duration,
            "instructions": instructions,
            "patient_name": format!("{} {}", patient_first, patient_last),
            "doctor_name": format!("Dr. {} {}", doctor_first, doctor_last),
            "appt_date": appt_date.format("%d %b %Y").to_string(),
            "admin_count": admin_count,
        }));
    }
    Ok(list)
}

/// Log a medication administration
pub async fn log_medication_administration(
    pool: &PgPool,
    prescription_id: Uuid,
    nurse_id: Uuid,
    remarks: Option<String>,
) -> Result<(), String> {
    sqlx::query!(
        r#"
        INSERT INTO medication_administration_log (prescription_id, administered_by_nurse_id, remarks)
        VALUES ($1, $2, $3)
        "#,
        prescription_id,
        nurse_id,
        remarks
    )
    .execute(pool)
    .await
    .map_err(|e| format!("Failed to log administration: {}", e))?;
    Ok(())
}

/// Cancel an appointment — only allowed if status is 'scheduled' and belongs to the patient
pub async fn cancel_patient_appointment(
    pool: &PgPool,
    appointment_id: Uuid,
    patient_id: Uuid,
) -> Result<(), String> {
    let result = sqlx::query!(
        r#"
        UPDATE appointment
        SET status = 'cancelled'::appointment_status, updated_at = NOW()
        WHERE id = $1 AND patient_id = $2 AND status = 'scheduled'
        "#,
        appointment_id,
        patient_id
    )
    .execute(pool)
    .await
    .map_err(|e| format!("DB error cancelling appointment: {}", e))?;

    if result.rows_affected() == 0 {
        Err("Cannot cancel: appointment not found, already checked-in, or does not belong to you.".to_string())
    } else {
        Ok(())
    }
}

/// Fetch doctor's appointments for the prescribe page (today + recent)
pub async fn get_doctor_prescribable_appointments(
    pool: &PgPool,
    doctor_id: Uuid,
) -> Result<Vec<serde_json::Value>, String> {
    let today_date = chrono::Local::now().date_naive();

    let rows = sqlx::query(
        r#"
        SELECT
            a.id,
            a.date,
            a.start_time,
            a.status::text AS status,
            p.first_name,
            p.last_name,
            (
                SELECT COUNT(*) FROM prescription pr WHERE pr.appointment_id = a.id
            ) AS rx_count
        FROM appointment a
        JOIN patient p ON a.patient_id = p.id
        WHERE a.doctor_id = $1
          AND a.date >= ($2::date - INTERVAL '7 days')
          AND a.status NOT IN ('cancelled', 'no_show')
        ORDER BY a.date DESC, a.start_time DESC
        "#,
    )
    .bind(doctor_id)
    .bind(today_date)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("DB error fetching prescribable appointments: {}", e))?;

    use sqlx::Row;
    let mut list = Vec::new();
    for row in rows {
        let id: Uuid            = row.get("id");
        let date: chrono::NaiveDate   = row.get("date");
        let start: chrono::NaiveTime  = row.get("start_time");
        let status: String            = row.get("status");
        let first: String             = row.get("first_name");
        let last: String              = row.get("last_name");
        let rx_count: i64             = row.get("rx_count");

        list.push(serde_json::json!({
            "id": id.to_string(),
            "appointment_date": date.format("%d %b %Y").to_string(),
            "is_today": date == today_date,
            "start_time": start.format("%I:%M %p").to_string(),
            "status": status,
            "patient_name": format!("{} {}", first, last),
            "rx_count": rx_count,
        }));
    }
    Ok(list)
}

/// Insert a single prescription for an appointment
pub async fn insert_prescription(
    pool: &PgPool,
    appointment_id: Uuid,
    doctor_id: Uuid,
    medicine_name: String,
    dosage: String,
    frequency: String,
    duration: String,
    instructions: Option<String>,
) -> Result<(), String> {
    sqlx::query!(
        r#"
        INSERT INTO prescription (appointment_id, prescribed_by_doctor_id, medicine_name, dosage, frequency, duration, instructions)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
        appointment_id,
        doctor_id,
        medicine_name,
        dosage,
        frequency,
        duration,
        instructions
    )
    .execute(pool)
    .await
    .map_err(|e| format!("Failed to insert prescription: {}", e))?;

    Ok(())
}