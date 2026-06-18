-- Enable UUID generation extension
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

---
--- 0. CUSTOM ENUM TYPES
---
-- Expanded to match your exact user personas
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
    password VARCHAR(255) NOT NULL, -- Hashed password via utils.rs
    role user_role NOT NULL,        -- Match your clean routing requirements
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
    room_type VARCHAR(100) NOT NULL, -- e.g., 'Consultation Room', 'Triage Station'
    location VARCHAR(255) NOT NULL   
);

---
--- 4. SCHEDULING & LIVE QUEUE (Receptionist & Patient Focus)
---
CREATE TABLE appointment (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    patient_id UUID NOT NULL REFERENCES patient(id) ON DELETE CASCADE,
    doctor_id UUID REFERENCES staff(id) ON DELETE SET NULL, -- Assigned doctor
    room_id UUID REFERENCES room(id) ON DELETE SET NULL,
    status appointment_status NOT NULL DEFAULT 'scheduled',
    date DATE NOT NULL,
    start_time TIME NOT NULL,
    end_time TIME NOT NULL,
    
    -- Live Clinic Queue tracking for Receptionists
    queue_number INT,
    check_in_time TIMESTAMP WITH TIME ZONE,
    
    created_by UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE triage_vitals (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    appointment_id UUID UNIQUE NOT NULL REFERENCES appointment(id) ON DELETE CASCADE,
    nurse_id UUID NOT NULL REFERENCES staff(id),
    blood_pressure VARCHAR(20),   -- e.g., '120/80'
    temperature NUMERIC(4, 2),    -- e.g., 36.5
    weight_kg NUMERIC(5, 2),      -- e.g., 70.5
    height_cm NUMERIC(5, 2),      -- e.g., 175.0
    recorded_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

---
--- 5. CLINICAL RECORDS (Doctor & Nurse Focus)
---
CREATE TABLE medical_records (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    patient_id UUID NOT NULL REFERENCES patient(id) ON DELETE CASCADE,
    appointment_id UUID UNIQUE NOT NULL REFERENCES appointment(id) ON DELETE CASCADE, 
    doctor_id UUID NOT NULL REFERENCES staff(id), -- Only doctors write diagnosis
    symptoms TEXT,
    diagnosis TEXT NOT NULL,                      -- Set by Doctor
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

-- Tracks Nurse administration actions cleanly without messing up prescription data
CREATE TABLE medication_administration_log (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    prescription_id UUID NOT NULL REFERENCES prescription(id) ON DELETE CASCADE,
    administered_by_nurse_id UUID NOT NULL REFERENCES staff(id),
    administered_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    remarks TEXT -- e.g., "Administered 1st dose, patient tolerated well"
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
    created_by_staff_id UUID REFERENCES staff(id) ON DELETE SET NULL, -- Usually Receptionist/Admin
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

---
--- 7. ADMIN IT SUPPORT TICKETS ("Get Help")
---
CREATE TABLE support_tickets (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    submitted_by_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    issue_description TEXT NOT NULL,
    status ticket_status NOT NULL DEFAULT 'open',
    admin_notes TEXT, -- Notes from the IT guy fixing the issue
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

---
--- 8. DATABASE INDEXES
---
CREATE INDEX idx_appointment_patient ON appointment(patient_id);
CREATE INDEX idx_appointment_date ON appointment(date);
CREATE INDEX idx_appointment_queue ON appointment(status, queue_number);
CREATE INDEX idx_med_records_patient ON medical_records(patient_id);
CREATE INDEX idx_tickets_status ON support_tickets(status);

-- Seed Triage Rooms (Nurses)
INSERT INTO room (room_name, room_type, location) VALUES
('Triage Station 1', 'triage', 'Level 1 Lobby'),
('Triage Station 2', 'triage', 'Level 1 Lobby');

-- Seed Consultation Rooms (Doctors)
INSERT INTO room (room_name, room_type, location) VALUES
('Room 101', 'consultation', 'Clinic Wing A'),
('Room 102', 'consultation', 'Clinic Wing A'),
('Room 103', 'consultation', 'Clinic Wing A'),
('Room 104', 'consultation', 'Clinic Wing B'),
('Room 105', 'consultation', 'Clinic Wing B'),
('Room 106', 'consultation', 'Clinic Wing B'),
('Room 107', 'consultation', 'Clinic Wing C'),
('Room 108', 'consultation', 'Clinic Wing C');