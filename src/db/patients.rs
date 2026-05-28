use sqlx::{PgPool, Postgres, Transaction};
use crate::models::user::{User, UserRole};
use crate::models::patient::CreatePatientProfile;
use crate::db::users::create_user;

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