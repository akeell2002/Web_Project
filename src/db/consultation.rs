use sqlx::{PgPool, Error};
use uuid::Uuid;
use crate::models::consultation::EncounterForm;

pub async fn finalize_consultation_and_bill(
    pool: &PgPool,
    appointment_id: Uuid,
    form: EncounterForm,
) -> Result<(), Error> {
    // 1. Begin the transaction
    let mut tx = pool.begin().await?;

    // 2. Fetch the required IDs dynamically from the active appointment
    let appointment = sqlx::query!(
        r#"
        SELECT patient_id, doctor_id 
        FROM appointment 
        WHERE id = $1
        "#,
        appointment_id
    )
    .fetch_one(&mut *tx)
    .await?;

    let patient_id = appointment.patient_id;
    let doctor_id = appointment.doctor_id.expect("A doctor must be assigned to the appointment.");

    // 3. Insert the core Medical Record
    sqlx::query!(
        r#"
        INSERT INTO medical_records (patient_id, appointment_id, doctor_id, symptoms, diagnosis, treatment_notes)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        patient_id,
        appointment_id,
        doctor_id,
        form.symptoms,
        form.diagnosis,
        form.treatment_notes
    )
    .execute(&mut *tx)
    .await?;

    // 4. Handle Prescriptions and compute fees
    let mut medicine_fee: f64 = 0.00;
    let consultation_fee: f64 = 50.00; // Base rate for the doctor visit

    if let Some(medicine) = form.medicine_name {
        if !medicine.trim().is_empty() {
            sqlx::query!(
                r#"
                INSERT INTO prescription (appointment_id, prescribed_by_doctor_id, medicine_name, dosage, frequency, duration, instructions)
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                "#,
                appointment_id,
                doctor_id,
                medicine,
                form.dosage.unwrap_or_default(),
                form.frequency.unwrap_or_default(),
                form.duration.unwrap_or_default(),
                form.instructions
            )
            .execute(&mut *tx)
            .await?;

            medicine_fee = 20.00; // Flat fee added if medicine is prescribed
        }
    }

    let total_amount = consultation_fee + medicine_fee;

    // 5. Generate the final Bill
    sqlx::query!(
        r#"
        INSERT INTO bills (patient_id, appointment_id, consultation_fee, medicine_fee, total_amount, payment_status)
        VALUES ($1, $2, $3::FLOAT8, $4::FLOAT8, $5::FLOAT8, 'unpaid')
        "#,
        patient_id,
        appointment_id,
        consultation_fee,
        medicine_fee,
        total_amount
    )
    .execute(&mut *tx)
    .await?;

    // 6. Mark the appointment as completed
    sqlx::query!(
        r#"
        UPDATE appointment 
        SET status = 'completed'::appointment_status 
        WHERE id = $1
        "#,
        appointment_id
    )
    .execute(&mut *tx)
    .await?;

    // 7. Commit everything to the database permanently
    tx.commit().await?;

    Ok(())
}