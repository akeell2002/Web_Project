use sqlx::PgPool;
use crate::models::medical_record::{MedicalRecord, CreateMedicalRecordDto};

// Insert a new medical record into the database
pub async fn create_medical_record(pool: &PgPool, record: CreateMedicalRecordDto) -> Result<MedicalRecord, sqlx::Error> {
    let new_record = sqlx::query_as!(
        MedicalRecord,
        r#"
        INSERT INTO medical_records (patient_id, doctor_id, diagnosis, notes, prescription)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#,
        record.patient_id,
        record.doctor_id,
        record.diagnosis,
        record.notes,
        record.prescription
    )
    .fetch_one(pool)
    .await?;

    Ok(new_record)
}

// Fetch all medical records for a specific patient
pub async fn get_records_by_patient(pool: &PgPool, patient_id: i32) -> Result<Vec<MedicalRecord>, sqlx::Error> {
    let records = sqlx::query_as!(
        MedicalRecord,
        r#"
        SELECT * FROM medical_records
        WHERE patient_id = $1
        ORDER BY record_date DESC
        "#,
        patient_id
    )
    .fetch_all(pool)
    .await?;

    Ok(records)
}