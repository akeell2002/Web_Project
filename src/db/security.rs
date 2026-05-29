use sqlx::{PgPool, Postgres, Executor, Row};
use uuid::Uuid;

use crate::models::access_log::AccessLogEntry;

pub async fn log_access_event<'e, E>(
    executor: E,
    actor_user_id: Option<Uuid>,
    actor_email: Option<&str>,
    action_type: &str,
    target_user_id: Option<Uuid>,
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