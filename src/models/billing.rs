use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal; // Clean native import

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PendingBillItem {
    pub bill_id: Uuid,
    pub appointment_id: Uuid,
    pub patient_first_name: String,
    pub patient_last_name: String,
    
    // Explicitly using Decimal types
    pub consultation_fee: Decimal,
    pub medicine_fee: Decimal,
    pub admission_fee: Decimal,
    pub total_amount: Decimal,
    
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct ProcessPaymentForm {
    pub bill_id: Uuid,
}