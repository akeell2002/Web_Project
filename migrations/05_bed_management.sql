-- Migration 05: Bed / Room Management
-- Adds status tracking to rooms and a transfer-request workflow.

-- 1. Extend room table with a manual status flag
--    (computed occupancy = active appointment today; maintenance = manual override)
ALTER TABLE room
    ADD COLUMN IF NOT EXISTS bed_status VARCHAR(20) NOT NULL DEFAULT 'available'
        CHECK (bed_status IN ('available', 'maintenance'));

-- 2. Transfer requests — any staff can create, only doctors can approve/reject
CREATE TABLE IF NOT EXISTS bed_transfers (
    id               UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    patient_id       UUID NOT NULL REFERENCES patient(id)  ON DELETE CASCADE,
    from_room_id     UUID          REFERENCES room(id)     ON DELETE SET NULL,
    to_room_id       UUID NOT NULL REFERENCES room(id)     ON DELETE CASCADE,
    requested_by_id  UUID NOT NULL REFERENCES staff(id),
    approved_by_id   UUID          REFERENCES staff(id),
    reason           TEXT,
    status           VARCHAR(20) NOT NULL DEFAULT 'pending'
                         CHECK (status IN ('pending', 'approved', 'rejected')),
    created_at       TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at       TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_bed_transfers_status  ON bed_transfers(status);
CREATE INDEX IF NOT EXISTS idx_bed_transfers_patient ON bed_transfers(patient_id);
