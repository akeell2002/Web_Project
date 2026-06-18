// src/db/patients.rs
use sqlx::{PgPool, Postgres, Transaction};
use crate::models::user::{User, UserRole};
use crate::models::patient::CreatePatientProfile;
use serde_json::json;
use uuid::Uuid;
use sqlx::Row;
use chrono::NaiveDate;

/// Registers a new patient user account and profile atomically using a transaction
pub async fn register_patient(
    pool: &PgPool,
    email: &str,
    raw_password: &str,
    profile: CreatePatientProfile,
) -> Result<User, String> {
    // 1. Begin a transaction block
    let mut tx: Transaction<'_, Postgres> = pool
        .begin()
        .await
        .map_err(|e| format!("Failed to start transaction: {}", e))?;

    // 2. Reuse your hashing & insertion logic inside the transaction context
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

    // 3. Insert into the profile table linking back to the user ID
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

    // 4. Commit transaction safely
    tx.commit()
        .await
        .map_err(|e| format!("Failed to commit transaction: {}", e))?;

    Ok(user)
}

/// Retrieve a simplified patient directory suitable for staff listing
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

/// Fetch a single patient's full profile + appointment/visit history for the detail page
pub async fn get_patient_detail(
    pool: &PgPool,
    patient_id: Uuid,
) -> Result<Option<serde_json::Value>, String> {
    // 1. Core profile (demographics + email)
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

    // 2. Appointment + visit history
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

        json!({
            "appointment_date": appt_date.format("%d %b %Y").to_string(),
            "start_time": start.format("%I:%M %p").to_string(),
            "end_time": end.format("%I:%M %p").to_string(),
            "status": status,
            "room": room.unwrap_or_else(|| "Waiting Area".to_string()),
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

    let visit_count = visits.len();
    Ok(Some(json!({
        "id": patient_id.to_string(),
        "full_name": format!("{} {}", first_name, last_name),
        "first_name": first_name,
        "last_name": last_name,
        "date_of_birth": dob.format("%d %b %Y").to_string(),
        "gender": gender,
        "phone": phone,
        "emergency_contact_name": ec_name,
        "emergency_contact_phone": ec_phone,
        "email": email,
        "registered_at": registered_at.format("%d %b %Y").to_string(),
        "visits": visits,
        "visit_count": visit_count,
    })))
}

/// Legacy fallback function to keep older handler references happy if they call it
pub async fn create_patient_profile(
    pool: &PgPool,
    first_name: &str,
    last_name: &str,
    date_of_birth: NaiveDate,
    gender: &str,
    phone_number: Option<&str>,
    email: &str,
) -> Result<(), String> {
    let profile = CreatePatientProfile {
        first_name: first_name.to_string(),
        last_name: last_name.to_string(),
        date_of_birth,
        gender: Some(gender.to_string()),
        phone_number: phone_number.map(|s| s.to_string()),
        emergency_contact_name: None,
        emergency_contact_phone: None,
    };
    register_patient(pool, email, "TemporaryPassword123!", profile).await.map(|_| ())
}