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