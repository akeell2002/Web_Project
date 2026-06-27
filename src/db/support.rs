use sqlx::{PgPool, Row};
use uuid::Uuid;
use serde::Serialize;

// Support ticket structure for serialization and database interaction
#[derive(Debug, Serialize)]
pub struct SupportTicket {
    pub id: Uuid,
    pub submitter_name: String,
    pub submitter_email: String,
    pub issue_description: String,
    pub status: String,
    pub reply_notes: Option<String>,
    pub replied_at: Option<String>,
    pub created_at: String,
}

// Insert a new public support ticket
pub async fn submit_ticket(
    pool: &PgPool,
    submitter_name: &str,
    submitter_email: &str,
    issue_description: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO support_tickets (submitter_name, submitter_email, issue_description)
        VALUES ($1, $2, $3)
        "#,
        submitter_name,
        submitter_email,
        issue_description,
    )
    .execute(pool)
    .await?;

    Ok(())
}

// Fetch all tickets ordered by newest first for the receptionist dashboard
pub async fn get_all_tickets(pool: &PgPool) -> Result<Vec<SupportTicket>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT
            id,
            COALESCE(submitter_name, 'Anonymous')  AS submitter_name,
            COALESCE(submitter_email, '')           AS submitter_email,
            issue_description,
            status::TEXT,
            reply_notes,
            replied_at,
            created_at
        FROM support_tickets
        ORDER BY created_at DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    let mut tickets = Vec::with_capacity(rows.len());
    for row in rows {
        let created_at: chrono::DateTime<chrono::Utc> = row.try_get("created_at")?;
        let replied_at: Option<chrono::DateTime<chrono::Utc>> = row.try_get("replied_at")?;

        tickets.push(SupportTicket {
            id: row.try_get("id")?,
            submitter_name: row.try_get("submitter_name")?,
            submitter_email: row.try_get("submitter_email")?,
            issue_description: row.try_get("issue_description")?,
            status: row.try_get("status")?,
            reply_notes: row.try_get("reply_notes")?,
            replied_at: replied_at.map(|t| t.format("%d %b %Y %H:%M").to_string()),
            created_at: created_at.format("%d %b %Y %H:%M").to_string(),
        });
    }

    Ok(tickets)
}

// Save a receptionist reply and mark the ticket as resolved
pub async fn reply_to_ticket(
    pool: &PgPool,
    ticket_id: Uuid,
    reply_notes: &str,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        UPDATE support_tickets
        SET reply_notes = $1,
            replied_at  = NOW(),
            status      = 'resolved'::ticket_status,
            updated_at  = NOW()
        WHERE id = $2
        "#,
        reply_notes,
        ticket_id,
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}
