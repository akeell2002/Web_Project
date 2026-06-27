use sqlx::PgPool;
use serde::Serialize;
use chrono;

#[derive(Debug, Serialize)]
pub struct ClinicAnalytics {
    pub total_patients:      i64,
    pub total_appointments:  i64,
    pub appts_today:         i64,
    pub appts_this_month:    i64,
    pub appts_completed:     i64,
    pub appts_cancelled:     i64,
    pub total_revenue:       String,
    pub revenue_this_month:  String,
    pub outstanding_bills:   i64,
    pub total_prescriptions: i64,
    pub total_doctors:       i64,
    pub total_nurses:        i64,
    pub total_receptionists: i64,
}

// Fetch all clinic-wide analytics metrics for the admin dashboard
pub async fn get_clinic_analytics(pool: &PgPool) -> Result<ClinicAnalytics, String> {
    let today = chrono::Local::now().date_naive();
    let now   = chrono::Local::now();

    let total_patients: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM patient")
        .fetch_one(pool).await.map_err(|e| e.to_string())?;

    let total_appointments: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM appointment")
        .fetch_one(pool).await.map_err(|e| e.to_string())?;

    let appts_today: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM appointment WHERE date = $1")
        .bind(today).fetch_one(pool).await.map_err(|e| e.to_string())?;

    let appts_this_month: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM appointment WHERE date_trunc('month', date) = date_trunc('month', $1::date)"
    ).bind(today).fetch_one(pool).await.map_err(|e| e.to_string())?;

    let appts_completed: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM appointment WHERE status = 'completed'"
    ).fetch_one(pool).await.map_err(|e| e.to_string())?;

    let appts_cancelled: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM appointment WHERE status IN ('cancelled', 'no_show')"
    ).fetch_one(pool).await.map_err(|e| e.to_string())?;

    let total_revenue: f64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(total_amount), 0)::float8 FROM bills WHERE payment_status = 'paid'"
    ).fetch_one(pool).await.map_err(|e| e.to_string())?;

    let revenue_this_month: f64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(total_amount), 0)::float8 FROM bills WHERE payment_status = 'paid' AND date_trunc('month', created_at) = date_trunc('month', $1::timestamptz)"
    ).bind(now).fetch_one(pool).await.map_err(|e| e.to_string())?;

    let outstanding_bills: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM bills WHERE payment_status = 'unpaid'"
    ).fetch_one(pool).await.map_err(|e| e.to_string())?;

    let total_prescriptions: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM prescription")
        .fetch_one(pool).await.map_err(|e| e.to_string())?;

    let total_doctors: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE role = 'doctor'")
        .fetch_one(pool).await.map_err(|e| e.to_string())?;

    let total_nurses: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE role = 'nurse'")
        .fetch_one(pool).await.map_err(|e| e.to_string())?;

    let total_receptionists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE role = 'receptionist'")
        .fetch_one(pool).await.map_err(|e| e.to_string())?;

    Ok(ClinicAnalytics {
        total_patients,
        total_appointments,
        appts_today,
        appts_this_month,
        appts_completed,
        appts_cancelled,
        total_revenue:      format!("{:.2}", total_revenue),
        revenue_this_month: format!("{:.2}", revenue_this_month),
        outstanding_bills,
        total_prescriptions,
        total_doctors,
        total_nurses,
        total_receptionists,
    })
}
