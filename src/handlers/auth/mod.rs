use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use uuid::Uuid;

// Token to reset password, keyed by user id
pub type ResetTokenStore = Arc<Mutex<HashMap<String, String>>>;

// Store for one-time verification codes for admin login, keyed by user id
pub type OtpStore = Arc<Mutex<HashMap<Uuid, OtpEntry>>>;

// Struct to hold OTP details
pub struct OtpEntry {
    pub code: String,
    pub email: String,
    pub expires_at: DateTime<Utc>,
    pub attempts: u8,
}

mod login;
mod register;
mod password;
mod dashboard;
mod profile;
mod otp;

// To export for use in other modules
pub use login::*;
pub use register::*;
pub use password::*;
pub use dashboard::*;
pub use profile::*;
pub use otp::*;
