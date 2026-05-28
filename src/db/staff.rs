use sqlx::{PgPool, Postgres, Transaction};
use crate::models::user::{User, UserRole};
use crate::models::staff::CreateStaffProfile;

/// Provisions a staff credentials and metadata mapping via an Admin execution block
pub async fn register_staff(
    pool: &PgPool,
    email: &str,
    raw_password: &str,
    role: UserRole, // Can be Doctor, Nurse, or Receptionist
    profile: CreateStaffProfile,
    ) -> Result<User, String> {
        if role == UserRole::Patient || role == UserRole::Admin {
            return Err("Invalid staff provisioning assignment context context error.".to_string());
        }

        let mut tx: Transaction<'_, Postgres> = pool
            .begin()
            .await
            .map_err(|e| format!("Transaction error: {}", e))?;

        let hashed_password = crate::utils::hash_password(raw_password)?;

        let user = sqlx::query_as!(
            User,
            r#"
            INSERT INTO users (email, password, role)
            VALUES ($1, $2, $3::user_role)
            RETURNING id, email, password, role as "role: UserRole", created_at as "created_at!", updated_at as "updated_at!"
            "#,
            email,
            hashed_password,
            role as UserRole
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| format!("Database user insertion failed: {}", e))?;

        sqlx::query!(
            r#"
            INSERT INTO staff (id, first_name, last_name, phone_number)
            VALUES ($1, $2, $3, $4)
            "#,
            user.id,
            profile.first_name,
            profile.last_name,
            profile.phone_number
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Database staff profile creation failed: {}", e))?;

        tx.commit()
            .await
            .map_err(|e| format!("Transaction commit failed: {}", e))?;

        Ok(user)
}