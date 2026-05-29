use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;
use crate::models::user::{User, UserRole};
use crate::models::staff::{CreateStaffProfile, StaffDashboardCounts, StaffDirectoryRow};
use crate::db::security::log_access_event;

fn role_label(role: &UserRole) -> &'static str {
    match role {
        UserRole::Admin => "Admin",
        UserRole::Doctor => "Doctor",
        UserRole::Nurse => "Nurse",
        UserRole::Receptionist => "Receptionist",
        UserRole::Patient => "Patient",
    }
}

fn display_name(first_name: Option<String>, last_name: Option<String>, email: &str, role: &UserRole) -> String {
    match (first_name, last_name) {
        (Some(first_name), Some(last_name)) => format!("{} {}", first_name, last_name),
        (Some(first_name), None) => first_name,
        (None, Some(last_name)) => last_name,
        _ if *role == UserRole::Admin => "System Admin".to_string(),
        _ => email.split('@').next().unwrap_or("Staff Member").replace('.', " "),
    }
}

/// Provisions staff credentials and metadata mapping via an Admin execution block
pub async fn register_staff(
    pool: &PgPool,
    email: &str,
    raw_password: &str,
    role: UserRole,
    profile: CreateStaffProfile,
    created_by_user_id: Option<Uuid>,
    created_by_email: Option<&str>,
    ) -> Result<User, String> {
        if role == UserRole::Patient {
            return Err("Invalid staff provisioning assignment context error.".to_string());
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
            role.clone() as UserRole
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

        let details = format!("Created {} account for {}.", role_label(&role), email);
        log_access_event(
            &mut *tx,
            created_by_user_id,
            created_by_email,
            "staff_account_created",
            Some(user.id),
            &user.email,
            role_label(&role),
            &details,
        )
        .await
        .map_err(|e| format!("Database access log insertion failed: {}", e))?;

        tx.commit()
            .await
            .map_err(|e| format!("Transaction commit failed: {}", e))?;

        Ok(user)
}

pub async fn get_staff_dashboard_counts(pool: &PgPool) -> Result<StaffDashboardCounts, String> {
    let counts = sqlx::query!(
        r#"
        SELECT
            COUNT(*) FILTER (WHERE role = 'admin'::user_role) AS "admins!",
            COUNT(*) FILTER (WHERE role = 'doctor'::user_role) AS "doctors!",
            COUNT(*) FILTER (WHERE role = 'nurse'::user_role) AS "nurses!",
            COUNT(*) FILTER (WHERE role = 'receptionist'::user_role) AS "receptionists!",
            COUNT(*) FILTER (
                WHERE role IN (
                    'admin'::user_role,
                    'doctor'::user_role,
                    'nurse'::user_role,
                    'receptionist'::user_role
                )
            ) AS "total_staff!"
        FROM users
        "#
    )
    .fetch_one(pool)
    .await
    .map_err(|e| format!("Database staff count query failed: {}", e))?;

    Ok(StaffDashboardCounts {
        total_staff: counts.total_staff,
        admins: counts.admins,
        doctors: counts.doctors,
        nurses: counts.nurses,
        receptionists: counts.receptionists,
    })
}

pub async fn get_staff_directory(pool: &PgPool, role_filter: Option<UserRole>) -> Result<Vec<StaffDirectoryRow>, String> {
    let rows = match role_filter {
        Some(role) => {
            let rows = sqlx::query!(
                r#"
                SELECT
                    u.id,
                    u.email,
                    u.role as "role: UserRole",
                    s.first_name as "first_name?",
                    s.last_name as "last_name?",
                    s.phone_number as "phone_number?",
                    u.created_at as "created_at!"
                FROM users u
                LEFT JOIN staff s ON s.id = u.id
                WHERE u.role = $1::user_role
                ORDER BY u.created_at DESC
                "#,
                role as UserRole
            )
            .fetch_all(pool)
            .await
            .map_err(|e| format!("Database staff directory query failed: {}", e))?;

            rows.into_iter()
                .map(|row| {
                    let display_name = display_name(row.first_name, row.last_name, &row.email, &row.role);

                    StaffDirectoryRow {
                        id: row.id,
                        email: row.email,
                        role: row.role,
                        display_name,
                        phone_number: row.phone_number,
                        created_at: row.created_at.format("%Y-%m-%d %H:%M").to_string(),
                    }
                })
                .collect::<Vec<_>>()
        }
        None => {
            let rows = sqlx::query!(
                r#"
                SELECT
                    u.id,
                    u.email,
                    u.role as "role: UserRole",
                    s.first_name as "first_name?",
                    s.last_name as "last_name?",
                    s.phone_number as "phone_number?",
                    u.created_at as "created_at!"
                FROM users u
                LEFT JOIN staff s ON s.id = u.id
                WHERE u.role IN (
                    'admin'::user_role,
                    'doctor'::user_role,
                    'nurse'::user_role,
                    'receptionist'::user_role
                )
                ORDER BY u.role ASC, u.created_at DESC
                "#
            )
            .fetch_all(pool)
            .await
            .map_err(|e| format!("Database staff directory query failed: {}", e))?;

            rows.into_iter()
                .map(|row| {
                    let display_name = display_name(row.first_name, row.last_name, &row.email, &row.role);

                    StaffDirectoryRow {
                        id: row.id,
                        email: row.email,
                        role: row.role,
                        display_name,
                        phone_number: row.phone_number,
                        created_at: row.created_at.format("%Y-%m-%d %H:%M").to_string(),
                    }
                })
                .collect::<Vec<_>>()
        }
    };

    Ok(rows)
}