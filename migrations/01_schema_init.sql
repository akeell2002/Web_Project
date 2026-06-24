-- Enable UUID generation extension
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

---
--- 0. CUSTOM ENUM TYPES
---
CREATE TYPE user_role AS ENUM ('admin', 'doctor', 'nurse', 'receptionist', 'patient');
CREATE TYPE bill_status AS ENUM ('unpaid', 'paid', 'partially_paid', 'refunded');
CREATE TYPE ticket_status AS ENUM ('open', 'in_progress', 'resolved');
CREATE TYPE appointment_status AS ENUM ('scheduled', 'checked_in', 'vitals_taken', 'completed', 'cancelled', 'no_show');

---
--- 1. USER SYSTEM
---
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    email VARCHAR(255) UNIQUE NOT NULL,
    password VARCHAR(255) NOT NULL,
    role user_role NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

---
--- 2. PROFILES
---
CREATE TABLE patient (
    id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    first_name VARCHAR(100) NOT NULL,
    last_name VARCHAR(100) NOT NULL,
    date_of_birth DATE NOT NULL,
    gender VARCHAR(50),
    phone_number VARCHAR(20),
    emergency_contact_name VARCHAR(150),
    emergency_contact_phone VARCHAR(20),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE staff (
    id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    first_name VARCHAR(100) NOT NULL,
    last_name VARCHAR(100) NOT NULL,
    phone_number VARCHAR(20),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

---
--- 3. INFRASTRUCTURE & ROOMS
---
CREATE TABLE room (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    room_name VARCHAR(100) NOT NULL,
    room_type VARCHAR(100) NOT NULL,
    location VARCHAR(255) NOT NULL,
    -- from 05_bed_management
    bed_status VARCHAR(20) NOT NULL DEFAULT 'available'
        CHECK (bed_status IN ('available', 'maintenance'))
);

---
--- 4. SCHEDULING & LIVE QUEUE
---
CREATE TABLE appointment (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    patient_id UUID NOT NULL REFERENCES patient(id) ON DELETE CASCADE,
    doctor_id UUID REFERENCES staff(id) ON DELETE SET NULL,
    room_id UUID REFERENCES room(id) ON DELETE SET NULL,
    status appointment_status NOT NULL DEFAULT 'scheduled',
    date DATE NOT NULL,
    start_time TIME NOT NULL,
    end_time TIME NOT NULL,
    queue_number INT,
    check_in_time TIMESTAMP WITH TIME ZONE,
    -- from 04_triage_priority
    priority_level INTEGER NOT NULL DEFAULT 4,
    created_by UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE triage_vitals (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    appointment_id UUID UNIQUE NOT NULL REFERENCES appointment(id) ON DELETE CASCADE,
    nurse_id UUID NOT NULL REFERENCES staff(id),
    blood_pressure VARCHAR(20),
    temperature NUMERIC(4, 2),
    weight_kg NUMERIC(5, 2),
    height_cm NUMERIC(5, 2),
    recorded_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

---
--- 5. CLINICAL RECORDS
---
CREATE TABLE medical_records (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    patient_id UUID NOT NULL REFERENCES patient(id) ON DELETE CASCADE,
    appointment_id UUID UNIQUE NOT NULL REFERENCES appointment(id) ON DELETE CASCADE,
    doctor_id UUID NOT NULL REFERENCES staff(id),
    symptoms TEXT,
    diagnosis TEXT NOT NULL,
    treatment_notes TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE prescription (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    appointment_id UUID NOT NULL REFERENCES appointment(id) ON DELETE CASCADE,
    prescribed_by_doctor_id UUID NOT NULL REFERENCES staff(id),
    medicine_name VARCHAR(255) NOT NULL,
    dosage VARCHAR(100) NOT NULL,
    frequency VARCHAR(100) NOT NULL,
    duration VARCHAR(100) NOT NULL,
    instructions TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE medication_administration_log (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    prescription_id UUID NOT NULL REFERENCES prescription(id) ON DELETE CASCADE,
    administered_by_nurse_id UUID NOT NULL REFERENCES staff(id),
    administered_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    remarks TEXT
);

---
--- 6. FINANCE & BILLING
---
CREATE TABLE bills (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    patient_id UUID NOT NULL REFERENCES patient(id) ON DELETE CASCADE,
    appointment_id UUID UNIQUE NOT NULL REFERENCES appointment(id) ON DELETE CASCADE,
    consultation_fee NUMERIC(10, 2) NOT NULL DEFAULT 0.00,
    medicine_fee NUMERIC(10, 2) NOT NULL DEFAULT 0.00,
    total_amount NUMERIC(10, 2) NOT NULL DEFAULT 0.00,
    payment_status bill_status NOT NULL DEFAULT 'unpaid',
    created_by_staff_id UUID REFERENCES staff(id) ON DELETE SET NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

---
--- 7. SUPPORT TICKETS
---
CREATE TABLE support_tickets (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    submitted_by_user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    submitter_name VARCHAR(255),
    submitter_email VARCHAR(255),
    issue_description TEXT NOT NULL,
    status ticket_status NOT NULL DEFAULT 'open',
    admin_notes TEXT,
    reply_notes TEXT,
    replied_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

---
--- 8. SYSTEM ACCESS LOGS
---
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

---
--- 9. BED TRANSFERS
---
CREATE TABLE bed_transfers (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    patient_id UUID NOT NULL REFERENCES patient(id) ON DELETE CASCADE,
    from_room_id UUID REFERENCES room(id) ON DELETE SET NULL,
    to_room_id UUID NOT NULL REFERENCES room(id) ON DELETE CASCADE,
    requested_by_id UUID NOT NULL REFERENCES staff(id),
    approved_by_id UUID REFERENCES staff(id),
    reason TEXT,
    status VARCHAR(20) NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'approved', 'rejected')),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

---
--- 10. INDEXES
---
CREATE INDEX idx_appointment_patient ON appointment(patient_id);
CREATE INDEX idx_appointment_date ON appointment(date);
CREATE INDEX idx_appointment_queue ON appointment(status, queue_number);
CREATE INDEX idx_med_records_patient ON medical_records(patient_id);
CREATE INDEX idx_tickets_status ON support_tickets(status);
CREATE INDEX idx_system_access_logs_created_at ON system_access_logs(created_at DESC);
CREATE INDEX idx_system_access_logs_action_type ON system_access_logs(action_type);
CREATE INDEX idx_bed_transfers_status ON bed_transfers(status);
CREATE INDEX idx_bed_transfers_patient ON bed_transfers(patient_id);

---
--- 11. SEED DATA
---
INSERT INTO room (room_name, room_type, location) VALUES
('Triage Station 1', 'triage', 'Level 1 Lobby'),
('Triage Station 2', 'triage', 'Level 1 Lobby'),
('Room 101', 'consultation', 'Clinic Wing A'),
('Room 102', 'consultation', 'Clinic Wing A'),
('Room 103', 'consultation', 'Clinic Wing A'),
('Room 104', 'consultation', 'Clinic Wing B'),
('Room 105', 'consultation', 'Clinic Wing B'),
('Room 106', 'consultation', 'Clinic Wing B'),
('Room 107', 'consultation', 'Clinic Wing C'),
('Room 108', 'consultation', 'Clinic Wing C');
