# Patient Management System

A full-stack enterprise web application built with **Rust**, **Actix Web**, **Tera**, and **PostgreSQL**. The system models a realistic hospital or clinic environment with five distinct user roles, a complete patient lifecycle, and advanced backend features including atomic conflict-safe appointment scheduling, role-based access control, audit logging, and an analytics dashboard.

---

## Default Accounts

| Role         | Email                   | Password      |
|-------------|-------------------------|---------------|
| Admin        | admin@clinic.com        | Password123!  |
| Doctor       | doctor@clinic.com       | Password123!  |
| Nurse        | nurse@clinic.com        | Password123!  |
| Receptionist | reception@clinic.com    | Password123!  |

Patients self-register at `/patient/register`, or staff can register them at `/staff/patients/add`.

---

## Technology Stack

| Layer       | Technology                                      |
|-------------|-------------------------------------------------|
| Language    | Rust 2024 edition                               |
| Web Server  | Actix Web 4                                     |
| Templating  | Tera (SSR)                                      |
| Database    | PostgreSQL via SQLx                             |
| Auth        | Actix Session (cookie-based) + Argon2 hashing  |
| Logging     | env_logger                                      |
| Config      | dotenv                                          |

---

## Architecture

```
src/
├── main.rs              — Actix bootstrap, routes, session/template setup
├── models/              — Domain structs (Patient, Staff, Appointment, Billing, …)
├── db/                  — SQLx database access layer
│   ├── patients.rs      — Patient CRUD + profile queries
│   ├── staff.rs         — Staff provisioning, directory, profile
│   ├── users.rs         — Auth, session, audit log
│   ├── appointments/    — Scheduling, triage, consultation sub-modules
│   ├── billing.rs       — Invoice and payment queries
│   ├── analytics.rs     — Clinic-wide aggregate statistics
│   ├── beds.rs          — Bed/room management and transfers
│   └── support.rs       — Help-desk ticket queries
├── handlers/            — HTTP request handlers (controller layer)
│   ├── auth/            — Login, logout, register, dashboard, profile, password reset
│   ├── admin/           — Staff onboarding, directory, analytics, security logs, support
│   ├── appointments/    — Scheduling, check-in, triage, consultation, prescriptions, meds
│   ├── patients.rs      — Patient CRUD handlers
│   ├── billing.rs       — Invoice and checkout handlers
│   └── beds.rs          — Bed management and transfer requests
├── utils.rs             — Password hashing helpers
templates/               — Tera SSR HTML templates
static/css/              — Frontend styles
migrations/              — SQL migrations (run automatically on startup)
```

---

## Setup

### Prerequisites

- Rust toolchain (stable)
- PostgreSQL (running locally)
- A `.env` file at the project root:

```env
DATABASE_URL=postgres://user:password@localhost/patient_db
SESSION_SECRET=<64-byte-secret-string>
```

### Run

```bash
# Apply migrations and start the server
cargo run

# Or with auto-reload during development
cargo watch -x run
```

The server starts at `http://127.0.0.1:8080`. Migrations run automatically on startup. On first launch, default staff accounts and 100 test patients are seeded.

---

## Implemented Modules

### Patient Registration & Management
- Self-registration via `/patient/register`
- Staff-assisted registration via `/staff/patients/add` (captures emergency contacts)
- Patient directory with search at `/staff/patients`
- Full patient profile with visit history, diagnoses, and prescriptions
- Patient profile edit (staff at `/staff/patients/{id}/edit`, patient self-edit at `/patient/profile`)
- Patient deletion (admin only, cascades all related records)
- Printable patient report at `/staff/patients/{id}/report`

### Appointment Scheduling & Conflict Resolution
- Patient self-booking at `/patient/appointments/book` with real-time slot availability grid
- Atomic INSERT prevents double-booking for both doctor and patient simultaneously
- 15-minute slot granularity across 09:00–17:00 clinic hours
- Appointment cancellation by patients
- Receptionist check-in with automatic queue number assignment
- No-show marking by receptionists

### Clinical Workflow (Triage → Consultation → Prescription → Medication)
- **Nurse triage**: record blood pressure, temperature, weight, height against appointment
- **Doctor queue**: view today's checked-in patients ordered by queue number
- **Doctor consultation**: write diagnosis, symptoms, treatment notes; issue prescriptions
- **Nurse medication administration**: log administered doses with remarks

### Billing
- Automatic bill creation on appointment completion
- Receptionist billing dashboard showing all outstanding invoices
- One-click payment checkout

### Bed & Room Management
- View current room occupancy status
- Create bed transfer requests for patients
- Doctor approval/rejection of transfer requests

### Admin Panel
- Staff onboarding (create Doctor / Nurse / Receptionist / Admin accounts)
- Staff directory with role filter
- Analytics dashboard: patient counts, appointment stats, revenue, prescription totals
- Security monitoring: paginated audit log of all login/logout/account-creation events

### Patient Portal
- Appointment booking with live slot grid
- Dashboard showing upcoming and historical appointments
- Medical history page: all visits with diagnosis, treatment notes, prescriptions
- Self-service profile edit (name, DOB, gender, phone, emergency contact)
- Password reset via email token (in-memory, resets on server restart)

### Support System
- Public support ticket submission (no login required)
- Admin support dashboard with ticket status management and reply functionality

---

## Database Schema

Defined across 5 migrations in `/migrations/`:

| Table                      | Purpose                                    |
|----------------------------|--------------------------------------------|
| `users`                    | Account credentials and role               |
| `patient`                  | Patient demographics                       |
| `staff`                    | Staff profile (name, phone)               |
| `room`                     | Rooms and their availability status        |
| `appointment`              | Scheduling, queue, check-in tracking       |
| `triage_vitals`            | Nurse-recorded vitals per appointment      |
| `medical_records`          | Doctor diagnosis and treatment notes       |
| `prescription`             | Medications prescribed per appointment     |
| `medication_administration_log` | Nurse dosage administration records  |
| `bills`                    | Invoice and payment status per appointment |
| `support_tickets`          | Help-desk tickets (public submission)      |
| `system_access_logs`       | Audit trail for login/logout/admin actions |
| `bed_transfers`            | Room transfer request workflow             |

---

## Route Summary

### Public
| Method | Route                     | Description                     |
|--------|---------------------------|---------------------------------|
| GET    | `/`                       | Landing page                    |
| GET    | `/support`                | Submit a support ticket         |
| GET    | `/patient/login`          | Patient login form              |
| POST   | `/patient/login`          | Patient login submit            |
| GET    | `/patient/register`       | Patient registration form       |
| POST   | `/patient/register`       | Patient registration submit     |
| GET    | `/forgot-password`        | Password reset request          |
| GET    | `/reset-password`         | Password reset form             |

### Patient Portal
| Method | Route                              | Description                       |
|--------|------------------------------------|-----------------------------------|
| GET    | `/patient/dashboard`               | Patient dashboard                 |
| GET    | `/patient/profile`                 | View + edit patient profile       |
| POST   | `/patient/profile`                 | Save profile changes              |
| GET    | `/patient/history`                 | Medical history (visits + Rx)     |
| GET    | `/patient/appointments/book`       | Appointment booking form          |
| POST   | `/patient/appointments/create`     | Book appointment                  |
| POST   | `/patient/appointments/{id}/cancel`| Cancel appointment                |

### Staff
| Method | Route                                          | Description                        |
|--------|------------------------------------------------|------------------------------------|
| GET    | `/staff/login`                                 | Staff login                        |
| GET    | `/staff/dashboard`                             | Staff dashboard                    |
| GET    | `/staff/patients`                              | Patient directory                  |
| GET    | `/staff/patients/add`                          | Add patient form                   |
| POST   | `/staff/patients/add`                          | Register new patient               |
| GET    | `/staff/patients/{id}`                         | Patient detail + visit history     |
| GET    | `/staff/patients/{id}/edit`                    | Edit patient form                  |
| POST   | `/staff/patients/{id}/edit`                    | Save patient edits                 |
| POST   | `/staff/patients/{id}/delete`                  | Delete patient (admin only)        |
| GET    | `/staff/patients/{id}/report`                  | Printable patient report           |
| GET    | `/staff/profile`                               | Staff profile view + edit          |
| POST   | `/staff/profile`                               | Save staff profile changes         |
| GET    | `/staff/doctor/queue`                          | Doctor daily queue                 |
| GET    | `/staff/doctor/consultation/{id}`              | Consultation form                  |
| POST   | `/staff/doctor/consultation/{id}`              | Submit consultation + Rx           |
| GET    | `/staff/doctor/prescribe`                      | Prescriptions list                 |
| POST   | `/staff/doctor/prescribe/{id}`                 | Issue prescription                 |
| GET    | `/staff/nurse/triage`                          | Nurse triage queue                 |
| POST   | `/staff/nurse/queue/triage/{id}`               | Submit triage vitals               |
| GET    | `/staff/nurse/medications`                     | Medication administration          |
| POST   | `/staff/nurse/medications/{id}/administer`     | Log dose administered              |
| GET    | `/staff/receptionist/reception`                | Reception desk queue               |
| POST   | `/staff/receptionist/queue/check_in/{id}`      | Check in patient                   |
| POST   | `/staff/receptionist/queue/no_show/{id}`       | Mark patient as no-show            |
| GET    | `/staff/receptionist/billing`                  | Billing dashboard                  |
| POST   | `/staff/receptionist/billing/checkout`         | Process payment                    |
| GET    | `/staff/beds`                                  | Bed management                     |
| POST   | `/staff/beds/transfer/request`                 | Request bed transfer               |
| POST   | `/staff/beds/transfer/{id}/approve`            | Approve transfer (doctor/admin)    |
| POST   | `/staff/beds/transfer/{id}/reject`             | Reject transfer                    |

### Admin
| Method | Route                     | Description                             |
|--------|---------------------------|-----------------------------------------|
| GET    | `/admin/dashboard`        | Admin dashboard with staff counts       |
| GET    | `/admin/analytics`        | Clinic analytics dashboard              |
| GET    | `/admin/security`         | Audit log / security monitoring         |
| GET    | `/admin/staff/onboard`    | Staff onboarding form                   |
| POST   | `/admin/staff/onboard`    | Create new staff account                |
| GET    | `/admin/staff`            | Staff directory                         |
| GET    | `/admin/support`          | Support ticket management               |
| POST   | `/admin/support/reply`    | Reply to a support ticket               |
