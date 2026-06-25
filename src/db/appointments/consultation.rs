use sqlx::PgPool;
use uuid::Uuid;
use chrono::Datelike;

/// Doctor's daily clinical queue with dynamic priority scoring
pub async fn get_doctor_daily_appointments(
    pool: &PgPool,
    doctor_id: Uuid,
    filter_mode: &str,
) -> Result<Vec<serde_json::Value>, String> {
    let mut list = Vec::new();
    let today = chrono::Local::now().date_naive();

    if filter_mode == "today" {
        let rows = sqlx::query!(
            r#"
            SELECT a.id, a.date, a.start_time, a.end_time,
                   a.status::text as "status!", a.queue_number,
                   a.priority_level,
                   (
                       CASE a.priority_level
                           WHEN 1 THEN 1000.0
                           WHEN 2 THEN 80.0  + (EXTRACT(EPOCH FROM (NOW() - COALESCE(a.check_in_time, NOW()))) / 60.0) * 2.0
                           WHEN 3 THEN 50.0  + (EXTRACT(EPOCH FROM (NOW() - COALESCE(a.check_in_time, NOW()))) / 60.0) * 1.0
                           WHEN 4 THEN 20.0  + (EXTRACT(EPOCH FROM (NOW() - COALESCE(a.check_in_time, NOW()))) / 60.0) * 0.5
                           ELSE        0.0   + (EXTRACT(EPOCH FROM (NOW() - COALESCE(a.check_in_time, NOW()))) / 60.0) * 0.2
                       END
                   )::FLOAT8 as "dynamic_score?",
                   p.first_name, p.last_name, p.date_of_birth, p.gender,
                   r.room_name          as "room_name?",
                   tv.blood_pressure,
                   tv.temperature::FLOAT8 as "temperature?",
                   tv.weight_kg::FLOAT8   as "weight_kg?",
                   tv.height_cm::FLOAT8   as "height_cm?"
            FROM appointment a
            JOIN patient p ON a.patient_id = p.id
            LEFT JOIN room         r  ON a.room_id      = r.id
            LEFT JOIN triage_vitals tv ON a.id           = tv.appointment_id
            WHERE a.doctor_id = $1 AND a.date = $2
              AND a.status NOT IN ('cancelled', 'no_show', 'admitted')
            ORDER BY
                CASE WHEN a.status IN ('vitals_taken', 'checked_in') THEN 1
                     WHEN a.status = 'scheduled' THEN 2
                     ELSE 3 END ASC,
                "dynamic_score?" DESC NULLS LAST,
                a.queue_number ASC
            "#,
            doctor_id,
            today
        )
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Failed to query daily clinical queue: {}", e))?;

        for row in rows {
            list.push(serde_json::json!({
                "id":               row.id,
                "appointment_date": row.date.format("%A, %b %d, %Y").to_string(),
                "is_today":         true,
                "start_time":       row.start_time.format("%I:%M %p").to_string(),
                "end_time":         row.end_time.format("%I:%M %p").to_string(),
                "status":           row.status,
                "queue_number":     row.queue_number,
                "priority_level":   row.priority_level,
                "dynamic_score":    format!("{:.1}", row.dynamic_score.unwrap_or(0.0)),
                "patient_name":     format!("{} {}", row.first_name, row.last_name),
                "patient_dob":      row.date_of_birth.to_string(),
                "patient_gender":   row.gender.unwrap_or_else(|| "Undisclosed".to_string()),
                "room":             row.room_name.unwrap_or_else(|| "Waiting Area".to_string()),
                "blood_pressure":   row.blood_pressure.unwrap_or_else(|| "--".to_string()),
                "temperature":      row.temperature.map(|t| format!("{:.1} °C", t)).unwrap_or_else(|| "--".to_string()),
                "weight":           row.weight_kg.map(|w| format!("{:.1} kg", w)).unwrap_or_else(|| "--".to_string()),
                "height":           row.height_cm.map(|h| format!("{:.1} cm", h)).unwrap_or_else(|| "--".to_string()),
            }));
        }
    }

    Ok(list)
}

/// Finalize a consultation: writes medical record, optional prescription, bill, and closes appointment
pub async fn finalize_consultation_and_bill(
    pool: &PgPool,
    appointment_id: Uuid,
    form: crate::models::appointment::EncounterForm,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    let appt = sqlx::query!(
        "SELECT patient_id, doctor_id FROM appointment WHERE id = $1",
        appointment_id
    )
    .fetch_one(&mut *tx)
    .await?;

    let patient_id = appt.patient_id;
    let doctor_id  = appt.doctor_id.expect("A doctor must be assigned to the appointment.");

    sqlx::query!(
        r#"
        INSERT INTO medical_records
            (patient_id, appointment_id, doctor_id, symptoms, diagnosis, treatment_notes)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        patient_id, appointment_id, doctor_id,
        form.symptoms, form.diagnosis, form.treatment_notes
    )
    .execute(&mut *tx)
    .await?;

    let mut medicine_fee: f64 = 0.0;
    let consultation_fee: f64 = 50.0;

    if let Some(medicine) = form.medicine_name {
        if !medicine.trim().is_empty() {
            sqlx::query!(
                r#"
                INSERT INTO prescription
                    (appointment_id, prescribed_by_doctor_id, medicine_name,
                     dosage, frequency, duration, instructions)
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                "#,
                appointment_id, doctor_id, medicine,
                form.dosage.unwrap_or_default(),
                form.frequency.unwrap_or_default(),
                form.duration.unwrap_or_default(),
                form.instructions
            )
            .execute(&mut *tx)
            .await?;
            medicine_fee = 20.0;
        }
    }

    sqlx::query!(
        r#"
        INSERT INTO bills
            (patient_id, appointment_id, consultation_fee, medicine_fee, total_amount, payment_status)
        VALUES ($1, $2, $3::FLOAT8, $4::FLOAT8, $5::FLOAT8, 'unpaid')
        "#,
        patient_id, appointment_id,
        consultation_fee, medicine_fee, consultation_fee + medicine_fee
    )
    .execute(&mut *tx)
    .await?;

    // Admission decision: when the doctor chose "yes" we admit the patient to a
    // free inpatient bed and keep the case open as 'admitted'. Otherwise we close
    // the encounter as usual ('completed') and release the consultation room.
    let admit = form.admit.as_deref() == Some("yes");

    if admit {
        // Find a free admission bed (not in maintenance, not already holding an
        // admitted patient). May be None if the ward is full.
        let bed = sqlx::query!(
            r#"
            SELECT id
            FROM room
            WHERE room_type = 'admission'
              AND bed_status <> 'maintenance'
              AND id NOT IN (
                  SELECT room_id FROM appointment
                  WHERE room_id IS NOT NULL AND status = 'admitted'
              )
            ORDER BY room_name
            LIMIT 1
            "#
        )
        .fetch_optional(&mut *tx)
        .await?;

        sqlx::query!(
            r#"
            UPDATE appointment
            SET status = 'admitted'::appointment_status, room_id = $2, updated_at = NOW()
            WHERE id = $1
            "#,
            appointment_id,
            bed.map(|b| b.id)
        )
        .execute(&mut *tx)
        .await?;
    } else {
        sqlx::query!(
            r#"
            UPDATE appointment
            SET status = 'completed'::appointment_status, room_id = NULL, updated_at = NOW()
            WHERE id = $1
            "#,
            appointment_id
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

/// One card per patient with all qualifying appointments nested (for prescribe page)
pub async fn get_doctor_prescribable_appointments(
    pool: &PgPool,
    doctor_id: Uuid,
) -> Result<Vec<serde_json::Value>, String> {
    use sqlx::Row;

    let rows = sqlx::query(
        r#"
        SELECT
            a.patient_id,
            a.id                                    AS appointment_id,
            a.date                                  AS appointment_date,
            a.start_time,
            a.status::TEXT                          AS status,
            p.first_name || ' ' || p.last_name      AS patient_name,
            (a.date = CURRENT_DATE)                 AS is_today,
            (SELECT COUNT(*)::INT FROM prescription rx WHERE rx.appointment_id = a.id) AS rx_count
        FROM appointment a
        JOIN patient p ON a.patient_id = p.id
        WHERE a.doctor_id = $1
          AND a.date >= CURRENT_DATE - INTERVAL '7 days'
          AND a.status NOT IN ('cancelled'::appointment_status, 'no_show'::appointment_status)
        ORDER BY a.patient_id, a.date DESC, a.start_time DESC
        "#,
    )
    .bind(doctor_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let mut patients: indexmap::IndexMap<Uuid, serde_json::Value> = indexmap::IndexMap::new();

    for r in &rows {
        let patient_id:   Uuid             = r.get("patient_id");
        let appt_id:      Uuid             = r.get("appointment_id");
        let date:         chrono::NaiveDate = r.get("appointment_date");
        let start:        chrono::NaiveTime = r.get("start_time");
        let status:       Option<String>   = r.get("status");
        let patient_name: Option<String>   = r.get("patient_name");
        let is_today:     Option<bool>     = r.get("is_today");
        let rx_count:     Option<i32>      = r.get("rx_count");

        let appt_entry = serde_json::json!({
            "id":               appt_id,
            "appointment_date": date.format("%d %b %Y").to_string(),
            "start_time":       start.format("%I:%M %p").to_string(),
            "status":           status.unwrap_or_default(),
            "is_today":         is_today.unwrap_or(false),
            "rx_count":         rx_count.unwrap_or(0),
        });

        let entry = patients.entry(patient_id).or_insert_with(|| serde_json::json!({
            "patient_id":   patient_id,
            "patient_name": patient_name.unwrap_or_default(),
            "appointments": [],
        }));

        entry["appointments"].as_array_mut().unwrap().push(appt_entry);
    }

    Ok(patients.into_values().collect())
}

/// Insert a standalone prescription (used from the prescribe page)
pub async fn insert_prescription(
    pool: &PgPool,
    appointment_id: Uuid,
    doctor_id: Uuid,
    medicine_name: &str,
    dosage: &str,
    frequency: &str,
    duration: &str,
    instructions: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO prescription
            (appointment_id, prescribed_by_doctor_id, medicine_name,
             dosage, frequency, duration, instructions)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
        appointment_id, doctor_id, medicine_name,
        dosage, frequency, duration, instructions,
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Fetch patient info + triage vitals + past diagnoses + past prescriptions for the consultation form
pub async fn get_consultation_patient_info(
    pool:           &PgPool,
    appointment_id: Uuid,
) -> Result<Option<serde_json::Value>, sqlx::Error> {
    let row = sqlx::query!(
        r#"
        SELECT
            p.id as patient_id,
            p.first_name, p.last_name, p.date_of_birth, p.gender,
            p.phone_number, p.emergency_contact_name, p.emergency_contact_phone,
            a.date, a.start_time, a.end_time,
            tv.blood_pressure,
            tv.temperature::FLOAT8 as "temperature?",
            tv.weight_kg::FLOAT8   as "weight_kg?",
            tv.height_cm::FLOAT8   as "height_cm?"
        FROM appointment a
        JOIN patient p ON a.patient_id = p.id
        LEFT JOIN triage_vitals tv ON tv.appointment_id = a.id
        WHERE a.id = $1
        "#,
        appointment_id
    )
    .fetch_optional(pool)
    .await?;

    let row = match row {
        Some(r) => r,
        None    => return Ok(None),
    };

    let patient_id = row.patient_id;

    let diagnoses = sqlx::query!(
        r#"
        SELECT mr.diagnosis, mr.symptoms, a.date
        FROM medical_records mr
        JOIN appointment a ON mr.appointment_id = a.id
        WHERE mr.patient_id = $1 AND mr.appointment_id != $2
        ORDER BY a.date DESC LIMIT 20
        "#,
        patient_id, appointment_id
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|d| serde_json::json!({
        "date":      d.date.format("%d %b %Y").to_string(),
        "diagnosis": d.diagnosis,
        "symptoms":  d.symptoms.unwrap_or_else(|| "-".to_string()),
    }))
    .collect::<Vec<_>>();

    let prescriptions = sqlx::query!(
        r#"
        SELECT pr.medicine_name, pr.dosage, pr.frequency, pr.duration,
               pr.instructions, a.date
        FROM prescription pr
        JOIN appointment a ON pr.appointment_id = a.id
        WHERE a.patient_id = $1 AND pr.appointment_id != $2
        ORDER BY a.date DESC LIMIT 20
        "#,
        patient_id, appointment_id
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|p| serde_json::json!({
        "date":          p.date.format("%d %b %Y").to_string(),
        "medicine_name": p.medicine_name,
        "dosage":        p.dosage,
        "frequency":     p.frequency,
        "duration":      p.duration,
        "instructions":  p.instructions.unwrap_or_else(|| "-".to_string()),
    }))
    .collect::<Vec<_>>();

    let dob = row.date_of_birth;
    let today = chrono::Local::now().date_naive();
    let mut age_years = today.year() - dob.year();
    if (today.month(), today.day()) < (dob.month(), dob.day()) { age_years -= 1; }
    let age = age_years.max(0) as u32;

    Ok(Some(serde_json::json!({
        "full_name":               format!("{} {}", row.first_name, row.last_name),
        "date_of_birth":           dob.format("%d %b %Y").to_string(),
        "age":                     age,
        "gender":                  row.gender.unwrap_or_else(|| "Not specified".to_string()),
        "phone":                   row.phone_number.unwrap_or_else(|| "-".to_string()),
        "emergency_contact_name":  row.emergency_contact_name.unwrap_or_else(|| "-".to_string()),
        "emergency_contact_phone": row.emergency_contact_phone.unwrap_or_else(|| "-".to_string()),
        "appointment_date":        row.date.format("%A, %d %b %Y").to_string(),
        "appointment_time":        format!("{} - {}", row.start_time.format("%I:%M %p"), row.end_time.format("%I:%M %p")),
        "blood_pressure":          row.blood_pressure.unwrap_or_else(|| "-".to_string()),
        "temperature":             row.temperature.map(|v| format!("{:.1} C", v)).unwrap_or_else(|| "-".to_string()),
        "weight_kg":               row.weight_kg.map(|v| format!("{:.1} kg", v)).unwrap_or_else(|| "-".to_string()),
        "height_cm":               row.height_cm.map(|v| format!("{:.0} cm", v)).unwrap_or_else(|| "-".to_string()),
        "past_diagnoses":          diagnoses,
        "past_prescriptions":      prescriptions,
    })))
}