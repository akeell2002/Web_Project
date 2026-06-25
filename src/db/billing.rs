use sqlx::{PgPool, Error};
use uuid::Uuid;
use crate::models::billing::PendingBillItem;

/// Fetches all invoices currently flagged as unpaid along with patient identity data
pub async fn get_unpaid_bills(pool: &PgPool) -> Result<Vec<PendingBillItem>, Error> {
    let bills = sqlx::query_as!(
        PendingBillItem,
        r#"
        SELECT 
            b.id as "bill_id!",
            b.appointment_id as "appointment_id!",
            p.first_name as "patient_first_name!",
            p.last_name as "patient_last_name!",
            b.consultation_fee as "consultation_fee!",
            b.medicine_fee as "medicine_fee!",
            b.admission_fee as "admission_fee!",
            b.total_amount as "total_amount!",
            b.created_at as "created_at"
        FROM bills b
        JOIN patient p ON b.patient_id = p.id
        WHERE b.payment_status = 'unpaid'::bill_status
        ORDER BY b.created_at DESC
        "#
    )
    .fetch_all(pool)
    .await?;

    Ok(bills)
}

/// Fetches all bills for a specific patient, joined with appointment date
pub async fn get_patient_bills(pool: &PgPool, patient_id: Uuid) -> Result<Vec<serde_json::Value>, Error> {
    let rows = sqlx::query!(
        r#"
        SELECT
            b.id,
            b.consultation_fee as "consultation_fee!",
            b.medicine_fee     as "medicine_fee!",
            b.admission_fee    as "admission_fee!",
            b.total_amount     as "total_amount!",
            b.payment_status::text as "payment_status!",
            b.created_at,
            a.date             as "appointment_date"
        FROM bills b
        JOIN appointment a ON b.appointment_id = a.id
        WHERE b.patient_id = $1
        ORDER BY b.created_at DESC
        "#,
        patient_id
    )
    .fetch_all(pool)
    .await?;

    let list = rows.into_iter().map(|r| {
        serde_json::json!({
            "id":               r.id,
            "appointment_date": r.appointment_date.format("%A, %b %d, %Y").to_string(),
            "consultation_fee": r.consultation_fee.to_string(),
            "medicine_fee":     r.medicine_fee.to_string(),
            "admission_fee":    r.admission_fee.to_string(),
            "total_amount":     r.total_amount.to_string(),
            "payment_status":   r.payment_status,
            "created_at":       r.created_at.map(|d| d.format("%b %d, %Y").to_string()).unwrap_or_else(|| "N/A".to_string()),
        })
    }).collect();

    Ok(list)
}

/// Transitions a bill status to paid securely inside a single database connection
pub async fn mark_bill_as_paid(pool: &PgPool, bill_id: Uuid, staff_user_id: Uuid) -> Result<(), Error> {
    sqlx::query!(
        r#"
        UPDATE bills
        SET payment_status = 'paid'::bill_status,
            created_by_staff_id = $1,
            updated_at = NOW()
        WHERE id = $2
        "#,
        staff_user_id,
        bill_id
    )
    .execute(pool)
    .await?;

    Ok(())
}