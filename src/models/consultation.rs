use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct EncounterForm {
    // Medical Record Fields
    pub symptoms: Option<String>,
    pub diagnosis: String,
    pub treatment_notes: Option<String>,
    
    // Prescription Fields
    pub medicine_name: Option<String>,
    pub dosage: Option<String>,
    pub frequency: Option<String>,
    pub duration: Option<String>,
    pub instructions: Option<String>,
}