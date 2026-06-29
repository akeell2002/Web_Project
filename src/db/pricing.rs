// Centeralized pricing rules for the clinic, so fees are easy to find and tweak in one place

// Consultation fee scales with the patient's triage priority
pub fn consultation_fee(priority_level: i32) -> f64 {
    match priority_level {
        1 => 180.0, // Emergency
        2 => 140.0, // Urgent
        3 => 100.0, // Semi-Urgent
        4 => 70.0,  // Routine
        _ => 50.0,  // Non-Urgent
    }
}

// Price for 1 prescribed medicine
pub fn medicine_price(name: &str) -> f64 {
    match name.trim().to_lowercase().as_str() {
        ""            => 0.0,
        "amoxicillin" => 30.0,
        "paracetamol" => 12.0,
        "ibuprofen"   => 15.0,
        "loratadine"  => 18.0,
        _             => 25.0,
    }
}

// Sum the price of every medicine in the prescription list
pub fn medicine_fee_total<I, S>(medicines: I) -> f64
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    medicines.into_iter().map(|m| medicine_price(m.as_ref())).sum()
}

// Admission fee per day
pub const ADMISSION_DAILY_RATE: f64 = 250.0;

// Total admission fee
pub fn admission_fee(nights: i64) -> f64 {
    let billable_nights = if nights < 1 { 1 } else { nights };
    ADMISSION_DAILY_RATE * billable_nights as f64
}
