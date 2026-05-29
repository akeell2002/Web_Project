use sqlx::{PgPool, Postgres, Transaction};
use crate::models::user::{User, UserRole};
use crate::models::patient::CreatePatientProfile;
use crate::db::users::create_user;
use serde_json::json;
use uuid::Uuid;
use sqlx::Row;

// Registers a new patient user account and profile atomically using a transaction
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

// Retrieve a simplified patient directory suitable for staff listing
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