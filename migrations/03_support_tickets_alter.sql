-- Allow anonymous (public) support ticket submissions
ALTER TABLE support_tickets
    ALTER COLUMN submitted_by_user_id DROP NOT NULL,
    ADD COLUMN IF NOT EXISTS submitter_name VARCHAR(255),
    ADD COLUMN IF NOT EXISTS submitter_email VARCHAR(255),
    ADD COLUMN IF NOT EXISTS reply_notes TEXT,
    ADD COLUMN IF NOT EXISTS replied_at TIMESTAMP WITH TIME ZONE;
