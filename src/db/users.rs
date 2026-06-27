use sqlx::{PgPool, Postgres, Executor, Row};
use crate::models::user::{User, UserRole, AccessLogEntry};
use crate::utils::{hash_password, verify_password};

// Find a user by their email
pub async fn find_user_by_email(pool: &PgPool, email: &str) -> Result<Option<User>, String> {
    let user = sqlx::query_as!(
        User,
        r#"
        SELECT id, email, password, role as "role: UserRole", created_at as "created_at!", updated_at as "updated_at!"
        FROM users
        WHERE email = $1
        "#,
        email
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Database error while retrieving user: {}", e))?;
    
    Ok(user)
}

// Authenticate user via email and password that is hashed and stored in db
pub async fn authenticate_user(pool: &PgPool, email: &str, password: &str) -> Result<Option<User>, String> {
    // 1. Locate the user profile
    let user = match find_user_by_email(pool, email).await? {
        Some(u) => u,
        None => return Ok(None), // If user not found return None
    };

    if verify_password(password, &user.password) {
        Ok(Some(user))
    } else {
        Ok(None) // If password mismatch
    }
}

// To seed default users for testing
pub async fn seed_default_staff_users(pool: &PgPool) -> Result<(), String> {
    let seed_accounts = [
        (UserRole::Admin, vec!["admin@clinic.com"]),
        (UserRole::Doctor, vec!["doctor@clinic.com"]),
        (UserRole::Nurse, vec!["nurse@clinic.com"]),
        (UserRole::Receptionist, vec!["receptionist@clinic.com"]),
        (UserRole::Patient, vec!["patient@clinic.com"]),
    ];

    for (role, emails) in seed_accounts {
        for email in emails {
            // all seeded acc use same pw
            let hashed_password = hash_password("faipi")?;
            let existing_user = find_user_by_email(pool, email).await?;

            match existing_user {
                Some(user) => {
                    // Update just in case
                    sqlx::query!(
                        r#"
                        UPDATE users
                        SET password = $2,
                            role = $3::user_role,
                            updated_at = CURRENT_TIMESTAMP
                        WHERE LOWER(email) = LOWER($1)
                        "#,
                        email,
                        hashed_password,
                        role.clone() as UserRole
                    )
                    .execute(pool)
                    .await
                    .map_err(|e| format!("Failed to refresh {}: {}", email, e))?;

                    let mut tx = pool.begin().await.map_err(|e| format!("Transaction error: {}", e))?;
                    
                    if role == UserRole::Patient {
                        sqlx::query!(
                            r#"
                            INSERT INTO patient (id, first_name, last_name, date_of_birth, gender, phone_number)
                            VALUES ($1, $2, $3, '1990-01-01', 'Not Specified', '000-000-0000')
                            ON CONFLICT (id) DO NOTHING
                            "#,
                            user.id,
                            "Seeded",
                            "Patient"
                        )
                        .execute(&mut *tx)
                        .await
                        .map_err(|e| format!("Failed to backfill patient profile: {}", e))?;
                    } else {
                        sqlx::query!(
                            r#"
                            INSERT INTO staff (id, first_name, last_name, phone_number)
                            VALUES ($1, $2, $3, '000-000-0000')
                            ON CONFLICT (id) DO NOTHING
                            "#,
                            user.id,
                            "Seeded",
                            match role {
                                UserRole::Admin => "Admin",
                                UserRole::Doctor => "Doctor",
                                UserRole::Nurse => "Nurse",
                                UserRole::Receptionist => "Receptionist",
                                _ => "Staff"
                            }
                        )
                        .execute(&mut *tx)
                        .await
                        .map_err(|e| format!("Failed to backfill staff profile: {}", e))?;
                    }
                    tx.commit().await.map_err(|e| format!("Failed to commit backfill: {}", e))?;

                    println!("Seeding layer: {} verified/refreshed.", email);
                }
                None => {
                    // Create seeded user from scratch
                    let mut tx = pool.begin().await.map_err(|e| format!("Transaction error: {}", e))?;

                    let new_user_id = uuid::Uuid::new_v4();
                    sqlx::query!(
                        r#"
                        INSERT INTO users (id, email, password, role)
                        VALUES ($1, $2, $3, $4::user_role)
                        "#,
                        new_user_id,
                        email.trim().to_lowercase(),
                        hashed_password,
                        role.clone() as UserRole
                    )
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| format!("Database seed user insertion failed: {}", e))?;

                    if role == UserRole::Patient {
                        sqlx::query!(
                            r#"
                            INSERT INTO patient (id, first_name, last_name, date_of_birth, gender, phone_number)
                            VALUES ($1, $2, $3, '1990-01-01', 'Not Specified', '000-000-0000')
                            "#,
                            new_user_id,
                            "Seeded",
                            "Patient"
                        )
                        .execute(&mut *tx)
                        .await
                        .map_err(|e| format!("Database seed patient profile creation failed: {}", e))?;
                    } else {
                        let role_label = match role {
                            UserRole::Admin => "Admin",
                            UserRole::Doctor => "Doctor",
                            UserRole::Nurse => "Nurse",
                            UserRole::Receptionist => "Receptionist",
                            _ => "Staff",
                        };

                        sqlx::query!(
                            r#"
                            INSERT INTO staff (id, first_name, last_name, phone_number)
                            VALUES ($1, $2, $3, '000-000-0000')
                            "#,
                            new_user_id,
                            "Seeded",
                            role_label
                        )
                        .execute(&mut *tx)
                        .await
                        .map_err(|e| format!("Database seed staff profile creation failed: {}", e))?;
                    }

                    // To audit the seeding process
                    let details = format!("System seeded {} account and profile for {}.", match role {
                        UserRole::Admin => "Admin",
                        UserRole::Doctor => "Doctor",
                        UserRole::Nurse => "Nurse",
                        UserRole::Receptionist => "Receptionist",
                        UserRole::Patient => "Patient",
                    }, email);

                    tx.commit().await.map_err(|e| format!("Transaction commit failed for {}: {}", email, e))?;

                    // Execute logging after a successful transaction commit
                    let _ = log_access_event(
                        pool,
                        None,
                        Some("system"),
                        "seed_account_created",
                        Some(new_user_id),
                        email,
                        match role {
                            UserRole::Admin => "Admin",
                            UserRole::Doctor => "Doctor",
                            UserRole::Nurse => "Nurse",
                            UserRole::Receptionist => "Receptionist",
                            UserRole::Patient => "Patient",
                        },
                        &details,
                    ).await;

                    println!("Seeding layer: {} and associated profile deployed.", email);
                }
            }
        }
    }

    Ok(())
}

// Update a user's password by email
pub async fn update_user_password(pool: &PgPool, email: &str, new_raw_password: &str) -> Result<bool, String> {
    let hashed = hash_password(new_raw_password)?;
    let result = sqlx::query!(
        "UPDATE users SET password = $1, updated_at = NOW() WHERE LOWER(email) = LOWER($2)",
        hashed,
        email
    )
    .execute(pool)
    .await
    .map_err(|e| format!("DB error updating password: {}", e))?;

    Ok(result.rows_affected() > 0)
}

// Log access events for auditing purposes
pub async fn log_access_event<'e, E>(
    executor: E,
    actor_user_id: Option<uuid::Uuid>,
    actor_email: Option<&str>,
    action_type: &str,
    target_user_id: Option<uuid::Uuid>,
    target_email: &str,
    target_role: &str,
    details: &str,
) -> Result<(), sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query(
        r#"
        INSERT INTO system_access_logs (
            actor_user_id,
            actor_email,
            action_type,
            target_user_id,
            target_email,
            target_role,
            details
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
    )
    .bind(actor_user_id)
    .bind(actor_email)
    .bind(action_type)
    .bind(target_user_id)
    .bind(target_email)
    .bind(target_role)
    .bind(details)
    .execute(executor)
    .await?;

    Ok(())
}

// Retrieve access logs
pub async fn get_access_logs(pool: &PgPool, limit: i64) -> Result<Vec<AccessLogEntry>, sqlx::Error> {
    let safe_limit = limit.clamp(1, 200);

    let rows = sqlx::query(
        r#"
        SELECT
            id,
            actor_user_id,
            actor_email,
            action_type,
            target_user_id,
            target_email,
            target_role,
            details,
            created_at
        FROM system_access_logs
        ORDER BY created_at DESC
        LIMIT $1
        "#,
    )
    .bind(safe_limit)
    .fetch_all(pool)
    .await?;

    let mut access_logs = Vec::with_capacity(rows.len());

    for row in rows {
        let created_at: chrono::DateTime<chrono::Utc> = row.try_get("created_at")?;

        access_logs.push(AccessLogEntry::from_parts(
            row.try_get("id")?,
            row.try_get("actor_user_id")?,
            row.try_get("actor_email")?,
            row.try_get("action_type")?,
            row.try_get("target_user_id")?,
            row.try_get("target_email")?,
            row.try_get("target_role")?,
            row.try_get("details")?,
            created_at,
        ));
    }

    Ok(access_logs)
}