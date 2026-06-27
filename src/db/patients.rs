// src/db/patients.rs
use sqlx::{PgPool, Postgres, Transaction};
use crate::models::user::{User, UserRole};
use crate::models::patient::CreatePatientProfile;
use serde_json::json;
use uuid::Uuid;
use sqlx::Row;
use chrono::NaiveDate;

// Registers a new patient user account and profile
pub async fn register_patient(
    pool: &PgPool,
    email: &str,
    raw_password: &str,
    profile: CreatePatientProfile,
) -> Result<User, String> {
    let mut tx: Transaction<'_, Postgres> = pool
        .begin()
        .await
        .map_err(|e| format!("Failed to start transaction: {}", e))?;

    let hashed_password = crate::utils::hash_password(raw_password)?;
    
    let user = sqlx::query_as!(
        User,
        r#"
        INSERT INTO users (email, password, role)
        VALUES ($1, $2, 'patient'::user_role)
        RETURNING id, email, password, role as "role: UserRole", created_at as "created_at!", updated_at as "updated_at!"
        "#,
        email,
        hashed_password
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| format!("Database error while inserting user: {}", e))?;

    sqlx::query!(
        r#"
        INSERT INTO patient (id, first_name, last_name, date_of_birth, gender, phone_number, emergency_contact_name, emergency_contact_phone)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
        user.id,
        profile.first_name,
        profile.last_name,
        profile.date_of_birth,
        profile.gender,
        profile.phone_number,
        profile.emergency_contact_name,
        profile.emergency_contact_phone
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Database error while creating patient profile: {}", e))?;

    let details = format!("Account created for {} for type patient",  email);
    tx.commit().await.map_err(|e| format!("Transaction commit failed for {}: {}", email, e))?;

    let _ = crate::db::users::log_access_event(
        pool,
        Some(user.id),
        Some(email),
        "patient_account_created",
        Some(user.id),
        email,
        "patient",
        &details,
    ).await;

    println!("Account created for {} for type patient",  email);    
    Ok(user)
}

// Retrieve  patient directory for staff dashboard
pub async fn get_patient_directory(pool: &PgPool) -> Result<Vec<serde_json::Value>, String> {
    let rows = sqlx::query(
        r#"
        SELECT u.id, u.email, p.first_name, p.last_name, p.date_of_birth, p.gender, p.phone_number
        FROM users u
        JOIN patient p ON u.id = p.id
        ORDER BY p.last_name, p.first_name
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("DB error fetching patients: {}", e))?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let id: Uuid = row.get::<Uuid, _>("id");
        let email: Option<String> = row.get::<Option<String>, _>("email");
        let first_name: String = row.get::<String, _>("first_name");
        let last_name: String = row.get::<String, _>("last_name");
        let date_of_birth: chrono::NaiveDate = row.get::<chrono::NaiveDate, _>("date_of_birth");
        let gender: Option<String> = row.get::<Option<String>, _>("gender");
        let phone: Option<String> = row.get::<Option<String>, _>("phone_number");

        out.push(json!({
            "id": id,
            "email": email,
            "first_name": first_name,
            "last_name": last_name,
            "date_of_birth": date_of_birth.to_string(),
            "gender": gender,
            "phone": phone,
            "blood_type": serde_json::Value::Null
        }));
    }

    Ok(out)
}

// Update patient profile info
pub async fn update_patient_profile(
    pool: &PgPool,
    email: &str,
    patient_id: Uuid,
    first_name: &str,
    last_name:  &str,
    date_of_birth: NaiveDate,
    gender:     Option<String>,
    phone_number: Option<String>,
    emergency_contact_name:  Option<String>,
    emergency_contact_phone: Option<String>,
) -> Result<(), String> {
    let tx: Transaction<'_, Postgres> = pool
        .begin()
        .await
        .map_err(|e| format!("Failed to start transaction: {}", e))?;

    sqlx::query!(
        r#"
        UPDATE patient
        SET first_name               = $2,
            last_name                = $3,
            date_of_birth            = $4,
            gender                   = $5,
            phone_number             = $6,
            emergency_contact_name   = $7,
            emergency_contact_phone  = $8,
            updated_at               = NOW()
        WHERE id = $1
        "#,
        patient_id,
        first_name,
        last_name,
        date_of_birth,
        gender,
        phone_number,
        emergency_contact_name,
        emergency_contact_phone
    )
    .execute(pool)
    .await
    .map_err(|e| format!("Failed to update patient profile: {}", e))?;

    // To audit the update process
    let details = format!("Account updated for {} for type patient",  email);
    tx.commit().await.map_err(|e| format!("Transaction commit failed for {}: {}", email, e))?;

    // Execute logging after a successful transaction commit
    let _ = crate::db::users::log_access_event(
        pool,
        Some(patient_id),
        Some(email),
        "patient_account_updated",
        Some(patient_id),
        email,
        "patient",
        &details,
    ).await;

    println!("Account updated for {} for type patient",  email);  
    Ok(())
}

// Delete a patients account
pub async fn delete_patient(pool: &PgPool, patient_id: Uuid, admin_email:&str, patient_email: &str,) -> Result<(), String> {
    let tx: Transaction<'_, Postgres> = pool
        .begin()
        .await
        .map_err(|e| format!("Failed to start transaction: {}", e))?;

    // To audit the deletion process
    let details = format!("Account {} deleted by {}.", patient_email, admin_email);
    tx.commit().await.map_err(|e| format!("Transaction commit failed for {}: {}", patient_email, e))?;

    // Execute logging after a successful transaction commit
    let _ = crate::db::users::log_access_event(
        pool,
        Some(patient_id),
        Some(patient_email),
        "patient_account_deleted",
        Some(patient_id),
        patient_email,
        "patient",
        &details,
    ).await;

    println!("Account deleted for {} successfully.", patient_email);

    sqlx::query!(
        "DELETE FROM users WHERE id = $1 AND role = 'patient'::user_role",
        patient_id
    )
    .execute(pool)
    .await
    .map_err(|e| format!("Failed to delete patient: {}", e))?;
    
      
    Ok(())
}

// Fetch a single patient's full profile and appointment history for the detail page
pub async fn get_patient_detail(
    pool: &PgPool,
    patient_id: Uuid,
) -> Result<Option<serde_json::Value>, String> {
    let profile_row = sqlx::query(
        r#"
        SELECT u.id, u.email, u.created_at,
               p.first_name, p.last_name, p.date_of_birth,
               p.gender, p.phone_number,
               p.emergency_contact_name, p.emergency_contact_phone
        FROM users u
        JOIN patient p ON u.id = p.id
        WHERE u.id = $1
        "#,
    )
    .bind(patient_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("DB error fetching patient profile: {}", e))?;

    let profile_row = match profile_row {
        Some(r) => r,
        None => return Ok(None),
    };

    let first_name: String = profile_row.get("first_name");
    let last_name: String  = profile_row.get("last_name");
    let dob: NaiveDate     = profile_row.get("date_of_birth");
    let gender: Option<String> = profile_row.get("gender");
    let phone: Option<String>  = profile_row.get("phone_number");
    let ec_name: Option<String>  = profile_row.get("emergency_contact_name");
    let ec_phone: Option<String> = profile_row.get("emergency_contact_phone");
    let email: Option<String>    = profile_row.get("email");
    let registered_at: chrono::DateTime<chrono::Utc> = profile_row.get("created_at");

    let visit_rows = sqlx::query(
        r#"
        SELECT
            a.id,
            a.date,
            a.start_time,
            a.end_time,
            a.status::text AS status,
            a.queue_number,
            r.room_name,
            s.first_name AS doctor_first,
            s.last_name  AS doctor_last,
            mr.diagnosis,
            mr.treatment_notes,
            pr.medicine_name,
            pr.dosage,
            pr.frequency,
            pr.duration
        FROM appointment a
        JOIN staff s ON a.doctor_id = s.id
        LEFT JOIN room r ON a.room_id = r.id
        LEFT JOIN medical_records mr ON mr.appointment_id = a.id
        LEFT JOIN prescription pr ON pr.appointment_id = a.id
        WHERE a.patient_id = $1
        ORDER BY a.date DESC, a.start_time DESC
        "#,
    )
    .bind(patient_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("DB error fetching patient visits: {}", e))?;

    let visits: Vec<serde_json::Value> = visit_rows.iter().map(|r| {
        let appt_date: NaiveDate           = r.get("date");
        let start: chrono::NaiveTime       = r.get("start_time");
        let end: chrono::NaiveTime         = r.get("end_time");
        let status: String                 = r.get("status");
        let room: Option<String>           = r.get("room_name");
        let queue: Option<i32>             = r.get("queue_number");
        let doc_first: String              = r.get("doctor_first");
        let doc_last: String               = r.get("doctor_last");
        let diagnosis: Option<String>      = r.get("diagnosis");
        let treatment: Option<String>      = r.get("treatment_notes");
        let medicine: Option<String>       = r.get("medicine_name");
        let dosage: Option<String>         = r.get("dosage");
        let frequency: Option<String>      = r.get("frequency");
        let duration: Option<String>       = r.get("duration");

        let room_display = room.unwrap_or_else(|| match status.as_str() {
            "completed" | "cancelled" | "no_show" | "admitted" => "—".to_string(),
            _ => "Waiting Area".to_string(),
        });

        json!({
            "appointment_date": appt_date.format("%d %b %Y").to_string(),
            "start_time": start.format("%I:%M %p").to_string(),
            "end_time": end.format("%I:%M %p").to_string(),
            "status": status,
            "room": room_display,
            "queue_number": queue,
            "doctor_name": format!("Dr. {} {}", doc_first, doc_last),
            "diagnosis": diagnosis,
            "treatment_notes": treatment,
            "medicine_name": medicine,
            "dosage": dosage,
            "frequency": frequency,
            "duration": duration,
        })
    }).collect();

    // Derived clinical summary data for the medical report
    let today = chrono::Local::now().date_naive();
    let age = today.years_since(dob).unwrap_or(0);

    let mut problem_list: Vec<String> = Vec::new();
    let mut medications: Vec<serde_json::Value> = Vec::new();
    let mut latest_diagnosis: Option<String> = None;
    for r in &visit_rows {
        let dx: Option<String> = r.get("diagnosis");
        if let Some(d) = dx {
            let d = d.trim().to_string();
            if !d.is_empty() {
                if latest_diagnosis.is_none() {
                    latest_diagnosis = Some(d.clone());
                }
                if !problem_list.contains(&d) {
                    problem_list.push(d);
                }
            }
        }
        let med: Option<String> = r.get("medicine_name");
        if let Some(m) = med {
            if !m.trim().is_empty() {
                medications.push(json!({
                    "medicine_name": m,
                    "dosage":    r.get::<Option<String>, _>("dosage"),
                    "frequency": r.get::<Option<String>, _>("frequency"),
                    "duration":  r.get::<Option<String>, _>("duration"),
                }));
            }
        }
    }

    let visit_count = visits.len();
    Ok(Some(json!({
        "id": patient_id.to_string(),
        "full_name": format!("{} {}", first_name, last_name),
        "first_name": first_name,
        "last_name": last_name,
        "date_of_birth": dob.format("%d %b %Y").to_string(),
        "date_of_birth_raw": dob.format("%Y-%m-%d").to_string(),
        "age": age,
        "gender": gender,
        "phone": phone,
        "emergency_contact_name": ec_name,
        "emergency_contact_phone": ec_phone,
        "email": email,
        "registered_at": registered_at.format("%d %b %Y").to_string(),
        "visits": visits,
        "visit_count": visit_count,
        "problem_list": problem_list,
        "medications": medications,
        "latest_diagnosis": latest_diagnosis,
    })))
}
