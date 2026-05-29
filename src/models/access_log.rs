use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct AccessLogEntry {
    pub id: Uuid,
    pub actor_user_id: Option<Uuid>,
    pub actor_email: Option<String>,
    pub action_type: String,
    pub target_user_id: Option<Uuid>,
    pub target_email: String,
    pub target_role: String,
    pub details: String,
    pub created_at: String,
}

impl AccessLogEntry {
    pub fn from_parts(
        id: Uuid,
        actor_user_id: Option<Uuid>,
        actor_email: Option<String>,
        action_type: String,
        target_user_id: Option<Uuid>,
        target_email: String,
        target_role: String,
        details: String,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            actor_user_id,
            actor_email,
            action_type,
            target_user_id,
            target_email,
            target_role,
            details,
            created_at: created_at.format("%Y-%m-%d %H:%M").to_string(),
        }
    }
}