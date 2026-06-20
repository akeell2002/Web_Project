mod scheduling;
mod triage;
mod consultation;

// Re-export everything so callers see no change: handlers::appointments::X still works
pub use scheduling::*;
pub use triage::*;
pub use consultation::*;
