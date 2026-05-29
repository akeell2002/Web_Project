CREATE TABLE system_access_logs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    actor_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    actor_email VARCHAR(255),
    action_type VARCHAR(100) NOT NULL,
    target_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    target_email VARCHAR(255) NOT NULL,
    target_role VARCHAR(50) NOT NULL,
    details TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_system_access_logs_created_at ON system_access_logs(created_at DESC);
CREATE INDEX idx_system_access_logs_action_type ON system_access_logs(action_type);