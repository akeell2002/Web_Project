# Patient Management System — Project Report

**Project type:** Full-stack hospital / clinic management web application
**Technology:** Rust · Actix-Web · Tera · PostgreSQL (SQLx)
**Architecture:** Layered, server-side-rendered web application with role-based access control

---

## 1. Introduction

The Patient Management System is a full-stack web application that models the complete operational workflow of a hospital or clinic. It supports the entire patient lifecycle — from self-registration and appointment booking, through reception check-in, nurse triage, doctor consultation, prescription, medication administration, and billing, to bed management and discharge — across five distinct, permission-scoped user roles (Admin, Doctor, Nurse, Receptionist, and Patient).

The application was deliberately engineered to go beyond a basic create-read-update-delete (CRUD) system. It demonstrates correctness under concurrency (atomic conflict-safe booking and advisory-lock queue numbering), a security-first design (role-based access control, Argon2 password hashing, two-factor authentication for administrators, and an immutable audit trail), and faithful adherence to a real clinical workflow modelled as a database-backed state machine.

This report documents the system's full feature set, separating **basic functionality** (the foundational requirements expected of a management system) from **advanced functionality** (the engineering features that distinguish this implementation), followed by the database design, security model, and overall workflow.

---

## 2. Technology Stack

| Layer | Technology | Purpose |
|-------|------------|---------|
| Language | Rust (2024 edition) | Memory safety and concurrency without a garbage collector |
| Web framework | Actix-Web 4 | High-performance asynchronous HTTP server |
| Templating | Tera 1 | Server-side rendering with automatic output escaping (XSS resistance) |
| Database | PostgreSQL via SQLx 0.7 | Relational store with native ENUMs, transactions, and advisory locks |
| Sessions | actix-session 0.9 | Signed cookie sessions |
| Password hashing | argon2 0.5 | Memory-hard password hashing |
| One-time codes | rand 0.8 | Generation of admin 2FA verification codes |
| Monetary types | rust_decimal / bigdecimal | Exact decimal arithmetic for billing |
| Dates/times | chrono 0.4 | Scheduling and timestamps |
| Identifiers | uuid (v4) | Non-sequential primary keys |
| Frontend | Bootstrap 5.3, custom CSS, vanilla JavaScript | Responsive user interface |
| Configuration | dotenv 0.15 | Environment-based configuration |

---

## 3. System Architecture

The application follows a clean layered architecture with strict separation between request handling, business logic, and data access.

```
Browser clients (Patient / Staff / Admin portals)
        │  HTTP
Actix-Web 4 · Session middleware · Routing (main.rs)
        │
Handler / controller layer (src/handlers/)
   — Role-based access guards (admin_only / staff_only)
        │
Data-access layer (src/db/, SQLx)
        │
PostgreSQL — 13 tables, 4 ENUM types, indexes, FK cascades
```

**Request lifecycle:** A request passes through the session middleware (which decodes the signed cookie), is dispatched by the router to a handler that enforces a role guard, calls into the data-access layer where all SQL resides, and is finally rendered into a Tera template and returned as HTML. The shared database connection pool, the compiled template engine, and the in-memory token stores are injected into every handler as shared application data.

---

## 4. Basic Features

These are the foundational capabilities expected of a patient management system.

### 4.1 User accounts and authentication
Account creation, login, and logout for all five roles. Patients can self-register; staff accounts are created by an administrator. Login is handled through two separate portals (patient and staff) with server-side validation.

### 4.2 Patient registration and management
Patients can self-register, and staff can register patients on their behalf (capturing emergency-contact details). The system provides a searchable patient directory, a full patient detail page showing visit history, profile editing for both staff and patients, and a printable patient report.

### 4.3 Appointment scheduling
Patients can book appointments through a booking form, reschedule existing appointments, and cancel appointments that have not yet started. Clinic hours are fixed at 09:00–17:00 using 15-minute slots.

### 4.4 Clinical record keeping
The system records triage vitals (blood pressure, temperature, weight, height), doctor diagnoses and treatment notes, prescriptions (medicine, dosage, frequency, duration), and medication-administration logs.

### 4.5 Billing
Bills are generated for each appointment, recording consultation, medicine, and admission fees, along with a payment status. Patients can view their bill history and reception staff can process payments.

### 4.6 Role-based dashboards
Each role is presented with a tailored dashboard: patients see upcoming and past appointments; staff see workflow-relevant queues; administrators see organisation-wide statistics.

### 4.7 Support tickets
A public support form (no login required) allows anyone to submit a help-desk ticket, which administrators can manage and reply to.

---

## 5. Advanced Features

These features demonstrate engineering depth beyond standard CRUD requirements.

### 5.1 Atomic, conflict-safe appointment booking
Booking is performed with a single `INSERT ... SELECT ... WHERE NOT EXISTS (...)` statement that rejects the write if either the doctor or the patient already holds an overlapping, non-cancelled appointment. Because validation and insertion occur atomically in one statement, there is no time-of-check-to-time-of-use (TOCTOU) race window — two concurrent requests for the same slot cannot both succeed.

### 5.2 Advisory-lock queue numbering
At check-in, each patient is assigned a sequential per-doctor queue number. The operation runs inside a transaction that first acquires a PostgreSQL advisory lock keyed on the doctor's identifier, forcing competing check-ins for the same doctor to serialise while check-ins for other doctors proceed unblocked. This makes the `MAX(queue_number) + 1` assignment safe under concurrency without locking the entire table.

### 5.3 Dynamic triage priority algorithm
The triage queue is ordered entirely in SQL by a weighted score that combines a clinical priority band (Emergency, Urgent, Semi-Urgent, Routine, Non-Urgent) with wait-time aging (one point per minute waited). Critical patients are seen first, while the aging component guarantees that lower-priority patients are not starved indefinitely.

### 5.4 Two-factor authentication for administrators
Administrator login is protected by an email one-time-password (OTP) second factor. After a correct password, the system generates a six-digit code, stores it with a five-minute expiry and an attempt limit, and issues a verification challenge. Crucially, the session is only granted a partial "pending" state at this stage — no role is assigned — so every administrative page remains locked by the existing role guard until the code is verified. Codes are single-use, expire after five minutes, and lock out after repeated failed attempts. (Email delivery is currently mocked to the server console, with a single send function designed to be swapped for a real SMTP integration.)

### 5.5 Role-based access control and authentication hardening
Every protected route is guarded server-side by role (`admin_only`, `staff_only`, or inline role checks). The two portals are strictly separated — patients cannot authenticate through the staff portal and vice versa. Login is enumeration-resistant: wrong-password and wrong-portal attempts return an identical generic error, preventing attackers from discovering which accounts exist. Passwords are hashed with Argon2 and are never serialised into responses.

### 5.6 Immutable audit trail
Every login, logout, and account create/update/delete event is written to an append-only audit log capturing the actor, target, action type, and details. The administrator security page renders these with human-readable labels and colour-coded action categories.

### 5.7 Centralised, priority-scaled pricing
All fee logic is centralised in a single pricing module (`src/db/pricing.rs`) so rates can be tuned in one place. Consultation fees scale with triage priority, medicine fees are summed per prescribed item, and admission fees accrue per night.

### 5.8 Compile-time-checked SQL
SQLx verifies queries against the live database schema at build time — including nullability and ENUM casts — so malformed SQL fails the build rather than surfacing as a runtime error.

### 5.9 Automatic migrations and idempotent seeding
On startup the application applies its schema migration and then seeds or refreshes the default accounts inside transactions, backfilling any missing patient or staff profile rows so the system is always in a known-good state after boot.

---

## 6. Functional Modules by Role

### 6.1 Patient portal
Self-registration and profile management; appointment booking with a live 15-minute slot grid; rescheduling and cancellation; a dashboard separating upcoming from historical appointments; a medical-history page listing visits, diagnoses, treatment notes, and prescriptions; bill history; and password reset.

### 6.2 Receptionist
Patient registration and directory access; check-in with automatic, advisory-lock-protected queue numbering and triage-room assignment; no-show marking; the billing dashboard listing outstanding invoices; and one-click payment checkout.

### 6.3 Nurse
Triage: recording vitals and advancing the appointment state, with automatic consultation-room assignment; and medication administration: logging administered doses with remarks against prescriptions.

### 6.4 Doctor
A daily queue of checked-in patients ordered by the dynamic priority algorithm; consultation entry (symptoms, diagnosis, treatment notes); prescription issuing; and approval or rejection of bed-transfer requests. Completing a consultation automatically generates a bill.

### 6.5 Administrator
Staff onboarding (creating Doctor, Nurse, Receptionist, or Admin accounts); staff directory with role filtering, editing, and deletion; an analytics dashboard (patient counts, appointment statistics, total and monthly revenue, outstanding bills, prescription totals, and staff headcounts); security/audit-log monitoring; support-ticket management; and patient deletion. Administrator access additionally requires the two-factor verification described in section 5.4.

### 6.6 Bed and room management (shared staff)
Room occupancy overview with computed status and current occupant; patient census and bed statistics; a bed-transfer request → approve/reject workflow (approval restricted to doctors and administrators); and patient discharge with admission-fee billing based on nights stayed.

---

## 7. Database Design

The schema is defined in a single consolidated migration applied automatically on startup. It comprises **13 tables**, **4 ENUM types**, supporting indexes, and seed data for 35 rooms.

### 7.1 ENUM types

| ENUM | Values |
|------|--------|
| `user_role` | admin, doctor, nurse, receptionist, patient |
| `bill_status` | unpaid, paid, partially_paid, refunded |
| `ticket_status` | open, in_progress, resolved |
| `appointment_status` | scheduled, checked_in, vitals_taken, completed, cancelled, no_show, admitted |

### 7.2 Core tables

The principal entities are `users` (credentials and role), `patient` and `staff` (profile rows whose primary key is also a foreign key to `users` in a 1:1 relationship with cascade delete), `room`, and `appointment` (the central scheduling and workflow entity). Clinical data is held in `triage_vitals`, `medical_records`, `prescription`, and `medication_administration_log`. Financial and operational data is held in `bills`, `bed_transfers`, `support_tickets`, and `system_access_logs`.

### 7.3 Referential integrity
`patient` and `staff` share their primary key with `users` via a 1:1 foreign key with `ON DELETE CASCADE`. Clinical records cascade from `appointment` and `patient`, while doctor, room, and audit references use `ON DELETE SET NULL` to preserve historical records. The `triage_vitals`, `medical_records`, and `bills` tables each hold a `UNIQUE` constraint on `appointment_id`, enforcing a strict one-to-one relationship with the appointment; prescriptions are one-to-many.

---

## 8. Security Model

The system applies defence in depth:

- **Password storage** uses Argon2 hashing with per-password salts; hashes are never serialised into responses.
- **Administrator two-factor authentication** adds an email OTP second factor, with the half-authenticated state secured by construction because no role is granted until the code is verified.
- **Session integrity** is provided by signed cookie sessions.
- **Access control** is enforced server-side on every protected route; unauthorised requests receive a 403 response and unauthenticated requests are redirected to the appropriate login.
- **User-enumeration resistance** is achieved by returning a single generic error for all authentication failures.
- **Auditability** is provided by the append-only access log.
- **Cross-site scripting** is mitigated by Tera's default output escaping.
- **Referential safety** is maintained through cascade and set-null foreign keys.

---

## 9. Application Workflow (State Machine)

The patient journey is driven by the `appointment_status` ENUM:

```
scheduled → checked_in → vitals_taken → completed
   │            │
   ├→ cancelled └→ no_show
                         (bed transfer) → admitted
```

| Step | Role | Action |
|------|------|--------|
| 1. Book appointment | Patient | Reserve a 15-minute slot (atomic, conflict-checked) |
| 2. Check-in | Receptionist | Assign queue number (advisory lock) and triage room |
| 3. Triage | Nurse | Record vitals; advance to `vitals_taken`; assign consultation room |
| 4. Consultation | Doctor | Diagnose, write notes, issue prescriptions; advance to `completed` |
| 5. Medication | Nurse | Log administered doses |
| 6. Billing | Receptionist | Collect payment; mark bill paid |
| (branch) Admission | Doctor / Admin | Bed transfer to `admitted`; discharge adds admission fees |

---

## 10. Feature Summary

| Category | Basic | Advanced |
|----------|-------|----------|
| Authentication | Login / logout, two portals | Admin email-OTP 2FA, enumeration resistance, Argon2 hashing |
| Patients | Registration, directory, profile editing, printable report | Cascade-safe deletion |
| Appointments | Booking, rescheduling, cancellation | Atomic conflict-safe booking (no TOCTOU race) |
| Queueing | Check-in, no-show | Advisory-lock per-doctor numbering, dynamic priority scoring |
| Clinical | Triage vitals, diagnosis, prescriptions, medication log | Workflow state machine via ENUM |
| Billing | Per-appointment bills, payment, history | Centralised priority-scaled pricing, automatic bill generation |
| Beds | Occupancy overview, transfers, discharge | Approval workflow, nights-based admission billing |
| Administration | Staff onboarding, directory, support tickets | Analytics dashboard, immutable audit log |
| Data layer | Relational schema, CRUD | Compile-time-checked SQL, auto-migration, idempotent seeding |

---

## 11. Conclusion

The Patient Management System satisfies the core requirements of a clinical management platform — multi-role accounts, patient and appointment management, clinical record keeping, and billing — while layering on advanced engineering features that address correctness, security, and real-world clinical fidelity. The combination of atomic conflict-safe scheduling, advisory-lock queueing, a dynamic triage algorithm, role-based access control with administrator two-factor authentication, an immutable audit trail, and compile-time-checked SQL demonstrates a system designed to be correct under concurrency and secure by default, rather than a simple CRUD prototype.

### Known limitations
The OTP and password-reset token stores are held in memory and are therefore cleared on server restart and unsuitable for multi-instance deployment. Email delivery is mocked to the console rather than sent. The default seeded password is for demonstration only and should be removed before any real deployment, and a strong session secret should be configured outside development.
