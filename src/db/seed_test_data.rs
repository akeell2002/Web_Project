/// seed_test_data.rs
/// Generates ~100 fake patients with appointments, triage vitals, and medical records.
/// Safe to call multiple times — skips patients that already exist.

use sqlx::PgPool;
use sqlx::Row;
use uuid::Uuid;
use chrono::Utc;
use crate::utils::hash_password;

const FIRST_NAMES_M: &[&str] = &[
    "Ahmad", "Muhammad", "Zulkifli", "Hafiz", "Farhan",
    "Rashid", "Aziz", "Kamal", "Syafiq", "Azrul",
    "Faizal", "Amin", "Haziq", "Nabil", "Rifqi",
    "Haris", "Danial", "Firdaus", "Izzat", "Luqman",
];
const FIRST_NAMES_F: &[&str] = &[
    "Nurul", "Siti", "Amirah", "Fatimah", "Zara",
    "Aishah", "Hajar", "Sofea", "Nadhira", "Izzati",
    "Liyana", "Syahira", "Nadia", "Afiqah", "Hidayah",
    "Alya", "Farhana", "Suraya", "Izzah", "Nabilah",
];
const LAST_NAMES: &[&str] = &[
    "Abdullah", "Rahman", "Ismail", "Hassan", "Ahmad",
    "Ibrahim", "Yusof", "Ghani", "Bakar", "Aziz",
    "Zainudin", "Othman", "Kadir", "Wahab", "Hamid",
    "Malik", "Noor", "Ali", "Razak", "Latif",
];
const GENDERS: &[&str] = &["Male", "Female"];
const BLOOD_TYPES: &[&str] = &["A+", "A-", "B+", "B-", "O+", "O-", "AB+", "AB-"];
const DIAGNOSES: &[&str] = &[
    "Hypertension",
    "Type 2 Diabetes Mellitus",
    "Upper Respiratory Tract Infection",
    "Acute Bronchitis",
    "Migraine",
    "Chronic Lower Back Pain",
    "Generalised Anxiety Disorder",
    "Iron Deficiency Anaemia",
    "Acute Gastroenteritis",
    "Urinary Tract Infection",
    "Hyperlipidaemia",
    "Asthma",
    "Hypothyroidism",
    "Dengue Fever",
    "Dyspepsia",
];
const SYMPTOMS: &[&str] = &[
    "Headache, dizziness, and elevated blood pressure",
    "Frequent urination, excessive thirst, fatigue",
    "Sore throat, runny nose, mild fever",
    "Productive cough, chest tightness, low-grade fever",
    "Throbbing headache, nausea, light sensitivity",
    "Lower back pain radiating to left leg",
    "Excessive worry, insomnia, restlessness",
    "Fatigue, pallor, shortness of breath on exertion",
    "Vomiting, diarrhoea, abdominal cramps",
    "Burning urination, frequency, lower abdominal pain",
    "No symptoms, found on routine lipid screen",
    "Wheezing, breathlessness, nocturnal cough",
    "Fatigue, weight gain, cold intolerance",
    "High fever, joint pain, positive NS1 antigen",
    "Epigastric pain, bloating, belching after meals",
];
const MEDICINES: &[&str] = &[
    "Amlodipine 5mg",
    "Metformin 500mg",
    "Amoxicillin 500mg",
    "Azithromycin 500mg",
    "Sumatriptan 50mg",
    "Diclofenac 50mg",
    "Escitalopram 10mg",
    "Ferrous Fumarate 200mg",
    "Oral Rehydration Salts",
    "Ciprofloxacin 500mg",
    "Atorvastatin 20mg",
    "Salbutamol Inhaler 100mcg",
    "Levothyroxine 50mcg",
    "Paracetamol 500mg",
    "Omeprazole 20mg",
];
const DOSAGES:    &[&str] = &["1 tablet", "2 tablets", "1 capsule", "2 puffs", "1 sachet"];
const FREQUENCY:  &[&str] = &["Once daily", "Twice daily", "Three times daily", "As needed"];
const DURATION:   &[&str] = &["7 days", "14 days", "30 days", "3 months", "Ongoing"];

pub async fn seed_test_patients(pool: &PgPool) -> Result<(), String> {
    // Check how many test patients already exist
    let existing: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM users WHERE email LIKE 'testpatient%@healthhub.test'"
    )
    .fetch_one(pool)
    .await
    .map_err(|e| format!("seed check failed: {}", e))?;

    if existing >= 100 {
        println!("Test data: {} fake patients already seeded, skipping.", existing);
        return Ok(());
    }

    println!("Test data: seeding fake patients (existing: {})…", existing);

    // Shared password hash for all test patients ("faipi")
    let hashed_pw = hash_password("faipi")?;

    // Grab the first doctor's user ID for appointment assignments
    let doctor_row = sqlx::query(
        r#"SELECT s.id FROM staff s
           JOIN users u ON u.id = s.id
           WHERE u.role = 'doctor'::user_role
           ORDER BY u.created_at
           LIMIT 1"#
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("doctor query: {}", e))?;

    let doctor_id: Option<Uuid> = doctor_row.map(|r| r.get("id"));

    // Grab nurse ID
    let nurse_row = sqlx::query(
        r#"SELECT s.id FROM staff s
           JOIN users u ON u.id = s.id
           WHERE u.role = 'nurse'::user_role
           ORDER BY u.created_at
           LIMIT 1"#
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("nurse query: {}", e))?;

    let nurse_id: Option<Uuid> = nurse_row.map(|r| r.get("id"));

    // Grab room IDs
    let room_rows = sqlx::query("SELECT id FROM room ORDER BY room_name")
        .fetch_all(pool)
        .await
        .map_err(|e| format!("room query: {}", e))?;

    let room_ids: Vec<Uuid> = room_rows.iter().map(|r| r.get("id")).collect();

    let total_to_seed = (100 - existing) as usize;

    // Appointment statuses for today's "live" patients (first 30)
    let live_statuses = [
        "checked_in", "checked_in", "checked_in",
        "vitals_taken", "vitals_taken", "vitals_taken",
        "completed", "completed",
    ];
    // Priority levels for triage
    let priorities = [1i32, 2, 2, 3, 3, 3, 4, 4, 4, 4];

    let start_idx = existing as usize;

    for i in 0..total_to_seed {
        let idx = start_idx + i;
        let gender = GENDERS[idx % 2];
        let first_name = if gender == "Male" {
            FIRST_NAMES_M[idx % FIRST_NAMES_M.len()]
        } else {
            FIRST_NAMES_F[idx % FIRST_NAMES_F.len()]
        };
        let last_name = LAST_NAMES[idx % LAST_NAMES.len()];
        let email = format!("testpatient{}@healthhub.test", idx + 1);
        let age_years: i64 = 18 + ((idx as i64 * 37) % 65);
        let dob = format!("{}-{:02}-{:02}",
            2025 - age_years,
            (idx % 12) + 1,
            (idx % 28) + 1,
        );
        let phone = format!("+60{}", 100000000 + (idx as u64 * 7654321) % 900000000);
        let blood_type = BLOOD_TYPES[idx % BLOOD_TYPES.len()];

        // 1. Insert user
        let new_user_id = Uuid::new_v4();
        let insert_result = sqlx::query(
            r#"INSERT INTO users (id, email, password, role)
               VALUES ($1, $2, $3, 'patient'::user_role)
               ON CONFLICT (email) DO NOTHING"#,
        )
        .bind(new_user_id)
        .bind(&email)
        .bind(&hashed_pw)
        .execute(pool)
        .await
        .map_err(|e| format!("insert user {}: {}", email, e))?;

        if insert_result.rows_affected() == 0 {
            continue; // already exists
        }

        // 2. Insert patient profile
        sqlx::query(
            r#"INSERT INTO patient (id, first_name, last_name, date_of_birth, gender, phone_number)
               VALUES ($1, $2, $3, $4::date, $5, $6)"#,
        )
        .bind(new_user_id)
        .bind(first_name)
        .bind(last_name)
        .bind(&dob)
        .bind(gender)
        .bind(&phone)
        .execute(pool)
        .await
        .map_err(|e| format!("insert patient {}: {}", email, e))?;

        // 3. For the first 30, create a TODAY appointment (live in clinic)
        if idx < 30 {
            let status = live_statuses[idx % live_statuses.len()];
            let priority = priorities[idx % priorities.len()];
            let hour = 8u32 + (idx as u32 % 9);
            let start_time = format!("{:02}:00:00", hour);
            let end_time   = format!("{:02}:00:00", hour + 1);
            let room_id: Option<Uuid> = if idx < room_ids.len() {
                Some(room_ids[idx % room_ids.len()])
            } else {
                None
            };

            let appt_id = Uuid::new_v4();
            sqlx::query(
                r#"INSERT INTO appointment
                       (id, patient_id, doctor_id, room_id, status, date,
                        start_time, end_time, queue_number, priority_level)
                   VALUES ($1, $2, $3, $4, $5::appointment_status, CURRENT_DATE,
                           $6::time, $7::time, $8, $9)"#,
            )
            .bind(appt_id)
            .bind(new_user_id)
            .bind(doctor_id)
            .bind(room_id)
            .bind(status)
            .bind(&start_time)
            .bind(&end_time)
            .bind((idx as i32) + 1)
            .bind(priority)
            .execute(pool)
            .await
            .map_err(|e| format!("insert appt {}: {}", email, e))?;

            // 4. Triage vitals for vitals_taken and completed
            if status == "vitals_taken" || status == "completed" {
                if let Some(nid) = nurse_id {
                    let bp   = format!("{}/{}", 110 + (idx % 40), 70 + (idx % 20));
                    let temp = 36.0 + ((idx % 30) as f64) * 0.1;
                    let wt   = 50.0 + ((idx % 50) as f64);
                    let ht   = 155.0 + ((idx % 30) as f64);

                    sqlx::query(
                        r#"INSERT INTO triage_vitals
                               (appointment_id, nurse_id, blood_pressure, temperature,
                                weight_kg, height_cm)
                           VALUES ($1, $2, $3, $4, $5, $6)
                           ON CONFLICT (appointment_id) DO NOTHING"#,
                    )
                    .bind(appt_id)
                    .bind(nid)
                    .bind(&bp)
                    .bind(temp)
                    .bind(wt)
                    .bind(ht)
                    .execute(pool)
                    .await
                    .map_err(|e| format!("insert vitals {}: {}", email, e))?;
                }
            }

            // 5. Medical records + prescription for completed
            if status == "completed" {
                if let Some(did) = doctor_id {
                    let diag_idx  = idx % DIAGNOSES.len();
                    let symp_idx  = idx % SYMPTOMS.len();
                    let med_idx   = idx % MEDICINES.len();

                    let mr_id = Uuid::new_v4();
                    sqlx::query(
                        r#"INSERT INTO medical_records
                               (id, patient_id, appointment_id, doctor_id,
                                symptoms, diagnosis, treatment_notes)
                           VALUES ($1, $2, $3, $4, $5, $6, $7)"#,
                    )
                    .bind(mr_id)
                    .bind(new_user_id)
                    .bind(appt_id)
                    .bind(did)
                    .bind(SYMPTOMS[symp_idx])
                    .bind(DIAGNOSES[diag_idx])
                    .bind("Patient counselled. Follow-up in 2 weeks.")
                    .execute(pool)
                    .await
                    .map_err(|e| format!("insert med_record {}: {}", email, e))?;

                    sqlx::query(
                        r#"INSERT INTO prescription
                               (appointment_id, prescribed_by_doctor_id, medicine_name,
                                dosage, frequency, duration, instructions)
                           VALUES ($1, $2, $3, $4, $5, $6, $7)"#,
                    )
                    .bind(appt_id)
                    .bind(did)
                    .bind(MEDICINES[med_idx])
                    .bind(DOSAGES[idx % DOSAGES.len()])
                    .bind(FREQUENCY[idx % FREQUENCY.len()])
                    .bind(DURATION[idx % DURATION.len()])
                    .bind("Take with food. Avoid alcohol.")
                    .execute(pool)
                    .await
                    .map_err(|e| format!("insert prescription {}: {}", email, e))?;
                }
            }
        }

        // 6. For patients 30-70, create PAST appointments (history data)
        if idx >= 30 && idx < 70 {
            let did = match doctor_id {
                Some(d) => d,
                None    => continue,
            };
            let days_ago = ((idx - 30) as i64 % 30) + 1;
            let past_date = Utc::now().date_naive() - chrono::Duration::days(days_ago);
            let appt_id   = Uuid::new_v4();
            let diag_idx  = idx % DIAGNOSES.len();
            let symp_idx  = idx % SYMPTOMS.len();
            let med_idx   = idx % MEDICINES.len();

            sqlx::query(
                r#"INSERT INTO appointment
                       (id, patient_id, doctor_id, status, date,
                        start_time, end_time, queue_number, priority_level)
                   VALUES ($1, $2, $3, 'completed'::appointment_status, $4,
                           '10:00:00', '10:30:00', $5, 4)"#,
            )
            .bind(appt_id)
            .bind(new_user_id)
            .bind(did)
            .bind(past_date)
            .bind((idx as i32) + 1)
            .execute(pool)
            .await
            .map_err(|e| format!("insert past appt {}: {}", email, e))?;

            sqlx::query(
                r#"INSERT INTO medical_records
                       (patient_id, appointment_id, doctor_id, symptoms, diagnosis, treatment_notes)
                   VALUES ($1, $2, $3, $4, $5, $6)"#,
            )
            .bind(new_user_id)
            .bind(appt_id)
            .bind(did)
            .bind(SYMPTOMS[symp_idx])
            .bind(DIAGNOSES[diag_idx])
            .bind("Patient counselled. Follow-up as required.")
            .execute(pool)
            .await
            .map_err(|e| format!("insert past med_record {}: {}", email, e))?;

            sqlx::query(
                r#"INSERT INTO prescription
                       (appointment_id, prescribed_by_doctor_id, medicine_name,
                        dosage, frequency, duration)
                   VALUES ($1, $2, $3, $4, $5, $6)"#,
            )
            .bind(appt_id)
            .bind(did)
            .bind(MEDICINES[med_idx])
            .bind(DOSAGES[idx % DOSAGES.len()])
            .bind(FREQUENCY[idx % FREQUENCY.len()])
            .bind(DURATION[idx % DURATION.len()])
            .execute(pool)
            .await
            .map_err(|e| format!("insert past rx {}: {}", email, e))?;
        }
    }

    println!("Test data: seeded {} fake patients successfully.", total_to_seed);
    Ok(())
}
