use std::sync::{Arc, Mutex};
use std::collections::HashMap;

// In-memory token store shared across password reset handlers
pub type ResetTokenStore = Arc<Mutex<HashMap<String, String>>>;

mod login;
mod register;
mod password;
mod dashboard;
mod profile;

// To export for use in other modules
pub use login::*;
pub use register::*;
pub use password::*;
pub use dashboard::*;
pub use profile::*;
