# Patient Management System

A full-stack, enterprise-grade **hospital / clinic management web application** built with **Rust**, **Actix-Web**, **Tera**, and **PostgreSQL**. The system models a realistic clinical environment end-to-end — from patient self-registration and appointment booking through reception check-in, nurse triage, doctor consultation, prescriptions, medication administration, billing, and bed management — under five distinct, permission-scoped user roles.

It goes well beyond a basic CRUD app: the backend is engineered to be **correct under concurrency** (atomic conflict-safe booking, advisory-lock queueing), **secure by default** (role-based access control, Argon2 password hashing, audit logging, enumeration-resistant authentication), and faithful to a real clinical workflow driven by a PostgreSQL `ENUM` state machine.

---

## Table of Contents

1. [Key Highlights](#key-highlights)
2. [Default Accounts](#default-accounts)
3. [Technology Stack](#technology-stack)
4. [System Architecture](#system-architecture)
5. [Project Structure](#project-structure)
6. [Database Schema](#database-schema)
7. [User Roles & Permissions](#user-roles--permissions)
8. [Feature Modules](#feature-modules)
9. [Advanced & Notable Technical Features](#advanced--notable-technical-features)
10. [Pricing Rules](#pricing-rules)
11. [Complete Route Reference](#complete-route-reference)
12. [Application Workflow & State Machine](#application-workflow--state-machine)
13. [Setup & Installation](#setup--installation)
14. [Configuration](#configuration)
15. [Security Considerations](#security-considerations)
16. [Development Notes & Known Limitations](#development-notes--known-limitations)

---

## Key Highlights

- **Five role-scoped portals** — Admin, Doctor, Nurse, Receptionist, and Patient, each with a tailored dashboard and strictly enforced permissions.
- **Complete patient lifecycle** — registration → booking → check-in → triage → consultation → prescription → medication → billing → discharge.
- **Concurrency-safe scheduling** — double-booking is impossible via a single atomic conditional `INSERT`; per-doctor queue numbers are assigned under a PostgreSQL advisory lock.
- **Dynamic triage priority** — an SQL weighted-scoring algorithm combining clinical priority with wait-time aging so urgent patients are seen first without starving routine ones.
- **Security first** — Argon2 password hashing, signed-cookie sessions, server-side role guards, portal separation, and a full audit trail of authentication and account events.
- **Compile-time-checked SQL** — SQLx verifies every query against the live schema at build time, eliminating an entire class of runtime database errors.
- **Server-side rendering** — Tera templates with output escaping keep the trust boundary on the server.
- **Single consolidated migration** — the entire schema (13 tables, 4 ENUM types, indexes, seed rooms) is applied automatically on startup.

---

## Default Accounts

On first launch, the application **seeds (or refreshes) five default accounts** via `seed_default_staff_users()`. All seeded accounts share the same password.

| Role         | Email                     | Password |
|--------------|---------------------------|----------|
| Admin        | `admin@clinic.com`        | `faipi`  |
| Doctor       | `doctor@clinic.com`       | `faipi`  |
| Nurse        | `nurse@clinic.com`        | `faipi`  |
| Receptionist | `receptionist@clinic.com` | `faipi`  |
| Patient      | `patient@clinic.com`      | `faipi`  |

> The seeding routine is **idempotent**: existing accounts are refreshed (password/role updated, `created_at` preserved) and missing `patient` / `staff` profile rows are backfilled inside a transaction, so the system is always in a known-good state after boot.

- **Staff portal login:** `/staff/login`
- **Patient portal login:** `/patient/login`
- Patients can self-register at `/patient/register`, or staff can register them at `/staff/patients/add` (which also captures emergency-contact details).

---

## Technology Stack

| Layer          | Technology                                          | Why |
|----------------|-----------------------------------------------------|-----|
| Language       | **Rust** (2024 edition)                             | Memory safety and fearless concurrency without a garbage collector |
| Web Server     | **Actix-Web 4**                                     | High-performance, async, actor-based HTTP server |
| Templating     | **Tera 1** (server-side rendering)                  | Output escaped by default → XSS-resistant |
| Database       | **PostgreSQL** via **SQLx 0.7**                     | Compile-time-checked SQL, native `ENUM`, transactions, advisory locks |
| Sessions       | **actix-session 0.9** (`CookieSessionStore`)        | Signed cookie sessions |
| Auth Hashing   | **argon2 0.5**                                      | Memory-hard, password-hashing-competition winner |
| Decimals       | **rust_decimal** / **bigdecimal**                   | Exact monetary arithmetic for billing |
| Dates/Times    | **chrono 0.4**                                      | Appointment scheduling and timestamps |
| IDs            | **uuid** (v4)                                       | Non-sequential primary keys |
| Static Files   | **actix-files 0.6**                                 | Serving CSS / JS / images |
| Frontend       | **Bootstrap 5.3** (CDN) + custom CSS + vanilla JS   | Responsive UI |
| Config         | **dotenv 0.15**                                     | Environment-based configuration |
| Logging        | **env_logger 0.10**                                 | Structured runtime logging |
| Errors         | **anyhow** / **thiserror**                          | Ergonomic error handling |
| Serialization  | **serde** / **serde_json**                          | Form parsing and JSON view models |

**Full dependency list** (`Cargo.toml`): `actix-web`, `actix-session` (cookie-session), `actix-files`, `tera`, `uuid` (v4 + serde), `sqlx` (runtime-tokio, tls-rustls, postgres, uuid, chrono, rust_decimal), `bigdecimal`, `rust_decimal`, `serde`, `serde_json`, `argon2`, `chrono`, `dotenv`, `anyhow`, `thiserror`, `env_logger`, `indexmap`.

---

## System Architecture

The application follows a clean **layered architecture** with a strict separation between request handling, business logic, and data access.

```
┌──────────────────────────────────────────────────────────────────┐
│  Browser Clients — Patient / Staff / Admin portals                │
│  (Bootstrap 5.3 + custom CSS + static/app.js, rendered by Tera)   │
└──────────────────────────────────────────────────────────────────┘
                              │  HTTP req/resp
┌──────────────────────────────────────────────────────────────────┐
│  Actix-Web 4  ·  SessionMiddleware (signed cookies)               │
│  Static file service  ·  Routing table (main.rs)                  │
└──────────────────────────────────────────────────────────────────┘
                              │
┌──────────────────────────────────────────────────────────────────┐
│  Handler / Controller layer  (src/handlers/)                      │
│  auth · appointments · admin · patients · billing · beds          │
│  ── Role-Based Access Control (admin_only / staff_only guards) ── │
│  ── Argon2 password hashing (utils.rs) ──                         │
└──────────────────────────────────────────────────────────────────┘
                              │  async queries
┌──────────────────────────────────────────────────────────────────┐
│  Data Access layer  (src/db/ — SQLx, compile-time checked)        │
│  patients · staff · users · appointments · billing · analytics    │
│  beds · support                                                   │
└──────────────────────────────────────────────────────────────────┘
                              │
┌──────────────────────────────────────────────────────────────────┐
│  PostgreSQL — 13 tables, 4 ENUM types, FK cascades, indexes       │
│  migrations/01_schema_init.sql (auto-run on startup)              │
└──────────────────────────────────────────────────────────────────┘
```

**Request lifecycle:**

1. A request enters Actix-Web and passes through `SessionMiddleware`, which decodes the signed session cookie.
2. The router (`main.rs`) dispatches to a **handler**, which enforces a role guard (`admin_only`, `staff_only`, or an inline role check) and reads identity from the session.
3. The handler calls into the **data-access layer** (`src/db/`), where all SQL lives, executing compile-time-checked queries against PostgreSQL.
4. Results are passed into a **Tera template** and rendered server-side; the HTML response is returned with appropriate cache-control headers.

The shared connection pool (`PgPool`, max 5 connections), the compiled Tera engine, and the in-memory password-reset token store are injected into every handler as Actix `web::Data`.

---

## Project Structure

```
.
├── Cargo.toml                  — Crate manifest & dependencies
├── .env                        — DATABASE_URL & SESSION_SECRET (not committed)
├── migrations/
│   └── 01_schema_init.sql      — Consolidated schema + ENUMs + indexes + seed rooms
├── static/
│   ├── app.js                  — Front-end interactivity (slot grid, forms)
│   ├── style.css               — Custom styling on top of Bootstrap
│   └── logo.png
├── templates/                  — Tera SSR templates
│   ├── index.html              — Landing page
│   ├── auth/                   — Login, register, password reset
│   ├── patient/                — Patient portal views
│   ├── staff/                  — Staff portal (doctor/nurse/receptionist) views
│   ├── admin/                  — Admin panel views
│   ├── support/                — Support ticket views
│   └── shared/                 — Shared layout & dashboard partials
└── src/
    ├── main.rs                 — Bootstrap: pool, migrations, seeding, routes, session
    ├── utils.rs                — Argon2 hash_password / verify_password
    ├── pricing.rs              — Centralized fee rules (consultation, medicine, admission)
    ├── models/                 — Domain structs & form DTOs
    │   ├── user.rs             — User, UserRole enum, login/register forms, audit log entry
    │   ├── patient.rs          — Patient + CreatePatientProfile
    │   ├── staff.rs            — Staff, onboarding forms, dashboard counts, directory rows
    │   ├── appointment.rs      — Appointment, UI slot, EncounterForm (consultation + Rx)
    │   └── billing.rs          — PendingBillItem, ProcessPaymentForm
    ├── db/                     — SQLx data-access layer
    │   ├── mod.rs              — create_db_pool()
    │   ├── users.rs            — Auth, seeding, audit log writes/reads
    │   ├── patients.rs         — Patient CRUD + profile queries
    │   ├── staff.rs            — Staff provisioning, directory, dashboard counts
    │   ├── appointments/
    │   │   ├── scheduling.rs   — Atomic booking, advisory-lock check-in, reschedule, cancel
    │   │   ├── triage.rs       — Dynamic priority queue + vitals recording
    │   │   └── consultation.rs — Diagnosis, prescriptions, auto bill creation
    │   ├── billing.rs          — Unpaid invoices, patient bills, mark-as-paid
    │   ├── analytics.rs        — Clinic-wide aggregate KPIs
    │   ├── beds.rs             — Bed overview/census, transfers, discharge billing
    │   └── support.rs          — Support ticket queries
    └── handlers/               — HTTP controllers
        ├── mod.rs              — get_display_name(), staff_only() guard
        ├── auth/               — login, register, dashboard, profile, password reset
        ├── admin/              — staff onboarding/directory, analytics, security, support
        ├── appointments/      — booking, triage, consultation, reception, meds
        ├── patients.rs         — Patient CRUD handlers + printable report
        ├── billing.rs          — Billing dashboard & checkout
        └── beds.rs             — Bed management, transfers, discharge
```

---

## Database Schema

Defined in **`migrations/01_schema_init.sql`** and applied automatically on startup via `sqlx::migrate!()`.

### ENUM Types

| ENUM                 | Values |
|----------------------|--------|
| `user_role`          | `admin`, `doctor`, `nurse`, `receptionist`, `patient` |
| `bill_status`        | `unpaid`, `paid`, `partially_paid`, `refunded` |
| `ticket_status`      | `open`, `in_progress`, `resolved` |
| `appointment_status` | `scheduled`, `checked_in`, `vitals_taken`, `completed`, `cancelled`, `no_show`, `admitted` |

### Tables

| Table | Key Columns | Purpose |
|-------|-------------|---------|
| `users` | `id` (UUID PK), `email` (unique), `password`, `role`, timestamps | Account credentials and role |
| `patient` | `id` (PK = FK → users, cascade), names, `date_of_birth`, `gender`, `phone_number`, `emergency_contact_name/phone` | Patient demographics |
| `staff` | `id` (PK = FK → users, cascade), names, `phone_number` | Staff profile |
| `room` | `id`, `room_name`, `room_type`, `location`, `bed_status` (`available`/`maintenance`) | Rooms (triage / consultation / admission) |
| `appointment` | `id`, `patient_id`, `doctor_id`, `room_id`, `status`, `date`, `start_time`, `end_time`, `queue_number`, `check_in_time`, `priority_level` (default 4), `created_by`, timestamps | Scheduling, queue, check-in & priority tracking |
| `triage_vitals` | `id`, `appointment_id` (unique), `nurse_id`, `blood_pressure`, `temperature`, `weight_kg`, `height_cm`, `recorded_at` | Nurse-recorded vitals per appointment |
| `medical_records` | `id`, `patient_id`, `appointment_id` (unique), `doctor_id`, `symptoms`, `diagnosis` (required), `treatment_notes` | Doctor diagnosis & consultation notes |
| `prescription` | `id`, `appointment_id`, `prescribed_by_doctor_id`, `medicine_name`, `dosage`, `frequency`, `duration`, `instructions` | Medications prescribed per appointment |
| `medication_administration_log` | `id`, `prescription_id`, `administered_by_nurse_id`, `administered_at`, `remarks` | Nurse dose administration records |
| `bills` | `id`, `patient_id`, `appointment_id` (unique), `consultation_fee`, `medicine_fee`, `admission_fee`, `total_amount`, `payment_status`, `created_by_staff_id`, timestamps | Invoice & payment status per appointment |
| `support_tickets` | `id`, `submitted_by_user_id` (nullable), `submitter_name/email`, `issue_description`, `status`, `admin_notes`, `reply_notes`, `replied_at`, timestamps | Help-desk tickets (public submission) |
| `system_access_logs` | `id`, `actor_user_id/email`, `action_type`, `target_user_id/email/role`, `details`, `created_at` | Audit trail for login/logout/admin actions |
| `bed_transfers` | `id`, `patient_id`, `from_room_id`, `to_room_id`, `requested_by_id`, `approved_by_id`, `reason`, `status` (`pending`/`approved`/`rejected`), timestamps | Room transfer request workflow |

### Indexes

`idx_appointment_patient`, `idx_appointment_date`, `idx_appointment_queue` (status, queue_number), `idx_med_records_patient`, `idx_tickets_status`, `idx_system_access_logs_created_at` (DESC), `idx_system_access_logs_action_type`, `idx_bed_transfers_status`, `idx_bed_transfers_patient`.

### Seed Rooms

The migration seeds **35 rooms**: 5 triage stations (Level 1 Lobby), 10 consultation rooms (Clinic Wings A–C), and 20 admission beds (Inpatient Wards A & B, generated via `generate_series`).

### Referential Integrity

`patient` and `staff` share their primary key with `users` via a `1:1` foreign key with `ON DELETE CASCADE`. Clinical records cascade from `appointment`/`patient`; `doctor_id`, `room_id`, and audit references use `ON DELETE SET NULL` to preserve history.

---

## User Roles & Permissions

| Capability | Patient | Receptionist | Nurse | Doctor | Admin |
|------------|:-------:|:------------:|:-----:|:------:|:-----:|
| Self-register & manage own profile | ✓ | — | — | — | — |
| Book / reschedule / cancel own appointments | ✓ | — | — | — | — |
| View own medical history & bills | ✓ | — | — | — | — |
| Patient directory & registration | — | ✓ | ✓ | ✓ | ✓ |
| Check-in / no-show / queue management | — | ✓ | — | — | — |
| Billing dashboard & payment checkout | — | ✓ | — | — | ✓ |
| Triage vitals & medication administration | — | — | ✓ | — | — |
| Consultation, diagnosis & prescriptions | — | — | — | ✓ | — |
| Bed management & transfer requests | — | ✓ | ✓ | ✓ | ✓ |
| Approve / reject bed transfers | — | — | — | ✓ | ✓ |
| Staff onboarding & directory | — | — | — | — | ✓ |
| Analytics dashboard | — | — | — | — | ✓ |
| Security / audit log | — | — | — | — | ✓ |
| Support ticket management & replies | — | — | — | — | ✓ |
| Delete patient (cascade) | — | — | — | — | ✓ |

Guards are enforced server-side: `admin_only()` (admin panel), `staff_only()` (any staff role), and inline role checks (e.g., billing allows `receptionist` or `admin`). Unauthorized staff requests receive `403 Forbidden`; unauthenticated requests are redirected to the appropriate login.

---

## Feature Modules

### Patient Registration & Management
- Self-registration via `/patient/register` (with confirm-password validation).
- Staff-assisted registration via `/staff/patients/add`, capturing emergency contacts.
- Searchable patient directory at `/staff/patients`.
- Full patient detail page with visit history, diagnoses, and prescriptions.
- Profile editing — staff at `/staff/patients/{id}/edit`, patient self-edit at `/patient/profile`.
- Patient deletion (admin only) cascading all related records.
- Printable patient report at `/staff/patients/{id}/report`.

### Appointment Scheduling
- Patient self-booking at `/patient/appointments/book` with a **live 15-minute slot grid** (09:00–17:00 clinic hours).
- **Atomic conflict-safe `INSERT`** prevents double-booking for both doctor and patient simultaneously.
- Appointment reschedule (`/patient/appointments/{id}/edit` → `/update`) with conflict re-validation.
- Appointment cancellation by patients (only while still `scheduled`).
- Receptionist check-in with automatic, **advisory-lock-protected queue numbering** and best-effort triage-station assignment.
- No-show marking by receptionists.

### Clinical Workflow (Triage → Consultation → Prescription → Medication)
- **Nurse triage:** record blood pressure, temperature, weight, and height; auto-assign a consultation room and advance status to `vitals_taken`.
- **Doctor queue:** today's checked-in patients ordered by the dynamic priority algorithm.
- **Doctor consultation:** write symptoms, diagnosis, and treatment notes; issue prescriptions; a bill is created automatically on completion.
- **Nurse medication administration:** log administered doses with remarks against prescriptions.

### Billing
- Automatic bill creation on appointment completion (consultation + medicine fees) and on discharge (plus admission fees).
- Receptionist billing dashboard listing all outstanding invoices.
- One-click payment checkout that marks bills as `paid`.
- Patient bill history at `/patient/bills`.

### Bed & Room Management
- Room occupancy overview with computed status (available / occupied / maintenance) and current patient.
- Patient census and bed statistics.
- Bed transfer **request → approve/reject** workflow (approval restricted to doctor/admin).
- Patient discharge with admission-fee billing based on nights stayed.

### Admin Panel
- Staff onboarding (create Doctor / Nurse / Receptionist / Admin accounts).
- Staff directory with role filter; edit and delete staff.
- **Analytics dashboard:** patient counts, appointment stats (today / month / completed / cancelled), total & monthly revenue, outstanding bills, prescription totals, and staff headcounts.
- **Security monitoring:** paginated audit log of login/logout/account events with human-readable labels and colour-coded action kinds.

### Patient Portal
- Dashboard separating upcoming vs. historical appointments.
- Medical history page (visits with diagnosis, treatment notes, prescriptions).
- Self-service profile editing.
- Bill history.
- Password reset via emailed token.

### Support System
- Public support-ticket submission at `/support` (no login required; submitter name/email optional).
- Admin support dashboard with status management and reply functionality.

---

## Advanced & Notable Technical Features

### 1. Atomic, Conflict-Safe Appointment Booking
Booking is performed with a single `INSERT ... SELECT ... WHERE NOT EXISTS (...) AND NOT EXISTS (...)` statement that rejects the write if **either** the doctor **or** the patient already has an overlapping, non-cancelled appointment. Because validation and insertion happen atomically in one statement, there is **no time-of-check-to-time-of-use (TOCTOU) race window** — concurrent requests for the same slot cannot both succeed.

### 2. Advisory-Lock Queue Numbering
Check-in assigns a sequential per-doctor queue number. The operation runs inside a transaction that first acquires `pg_advisory_xact_lock(doctor_id)`, forcing competing check-ins **for the same doctor** to serialize single-file while check-ins for **other doctors proceed unblocked**. This makes `SELECT MAX(queue_number) + 1` safe under concurrency without locking the whole table.

### 3. Dynamic Triage Priority Algorithm
The triage queue is ordered entirely in SQL by a weighted score:

```
score = priority_band_weight  +  minutes_waited
        (Emergency 1000, Urgent 500, Semi-Urgent 200, Routine 50, Non-Urgent 10)
        + EXTRACT(EPOCH FROM (NOW() - check_in_time)) / 60
```

Critical patients are seen first, while **wait-time aging** (one point per minute waited) guarantees lower-priority patients are not starved indefinitely.

### 4. Role-Based Access Control & Authentication Security
- Server-side guards (`admin_only`, `staff_only`, inline checks) on every protected route.
- **Portal separation:** patients cannot log in via the staff portal and vice versa.
- **Enumeration-resistant login:** wrong-portal and wrong-password attempts return the *same* generic error, so attackers cannot discover which accounts exist.
- Passwords hashed with **Argon2** (`utils.rs`).
- Session caches the user's display name to avoid extra queries and reflect name changes immediately.

### 5. Immutable Audit Trail
Every login, logout, and account create/update/delete is written to `system_access_logs` with actor, target, action type, and details. The admin security page renders these with humanized labels (e.g. `staff_account_deleted` → "Staff Deleted") and colour groups (created / updated / deleted / admitted / discharged / access).

### 6. Centralized, Priority-Scaled Pricing
All fee logic lives in `pricing.rs` so rates are tuned in one place. Consultation fees scale with triage priority; medicine fees are summed per prescribed item; admission fees accrue per night.

### 7. Compile-Time-Checked SQL
SQLx's `query!` / `query_as!` macros verify every query against the live database schema at compile time, including nullability and `ENUM` casts — malformed SQL fails the build rather than surfacing at runtime.

### 8. Automatic Migrations & Idempotent Seeding
On startup the app runs all migrations and then seeds/refreshes the five default accounts inside transactions, backfilling any missing `patient`/`staff` profile rows.

---

## Pricing Rules

Defined in `src/pricing.rs`.

**Consultation fee** (by triage priority level):

| Priority | Level | Fee |
|----------|-------|-----|
| Emergency | 1 | $180 |
| Urgent | 2 | $140 |
| Semi-Urgent | 3 | $100 |
| Routine | 4 (default) | $70 |
| Non-Urgent | 5+ | $50 |

**Medicine prices** (per item; unknown medicines default to $25):

| Medicine | Price |
|----------|-------|
| Amoxicillin | $30 |
| Paracetamol | $12 |
| Ibuprofen | $15 |
| Loratadine | $18 |

**Admission fee:** `$250 per night` (minimum 1 billable night).

---

## Complete Route Reference

### Public
| Method | Route | Description |
|--------|-------|-------------|
| GET | `/` | Landing page |
| GET | `/support` | Submit a support ticket |
| POST | `/support/submit` | Submit support ticket |
| GET | `/patient/login` | Patient login form |
| POST | `/patient/login` | Patient login submit |
| GET | `/patient/register` | Patient registration form |
| POST | `/patient/register` | Patient registration submit |
| GET / POST | `/forgot-password` | Password reset request |
| GET / POST | `/reset-password` | Password reset form & submit |
| GET | `/logout` | Log out (redirects to relevant login) |

### Patient Portal
| Method | Route | Description |
|--------|-------|-------------|
| GET | `/patient/dashboard` | Upcoming & historical appointments |
| GET / POST | `/patient/profile` | View & save profile |
| GET | `/patient/history` | Medical history (visits + Rx) |
| GET | `/patient/bills` | Bill history |
| GET | `/patient/appointments/book` | Booking form with live slot grid |
| POST | `/patient/appointments/create` | Book appointment |
| GET | `/patient/appointments/{id}/edit` | Reschedule form |
| POST | `/patient/appointments/{id}/update` | Submit reschedule |
| POST | `/patient/appointments/{id}/cancel` | Cancel appointment |

### Staff (shared)
| Method | Route | Description |
|--------|-------|-------------|
| GET / POST | `/staff/login` | Staff login |
| GET | `/staff/dashboard` | Staff dashboard |
| GET | `/staff/patients` | Patient directory |
| GET / POST | `/staff/patients/add` | Add / register patient |
| GET | `/staff/patients/{id}` | Patient detail + visit history |
| GET / POST | `/staff/patients/{id}/edit` | Edit patient |
| POST | `/staff/patients/{id}/delete` | Delete patient (admin only) |
| GET | `/staff/patients/{id}/report` | Printable patient report |
| GET / POST | `/staff/profile` | Staff profile view & save |
| GET | `/staff/beds` | Bed management |
| POST | `/staff/beds/transfer/request` | Request bed transfer |
| POST | `/staff/beds/transfer/{id}/approve` | Approve transfer (doctor/admin) |
| POST | `/staff/beds/transfer/{id}/reject` | Reject transfer |
| POST | `/staff/beds/{id}/discharge` | Discharge patient (+ admission billing) |

### Doctor
| Method | Route | Description |
|--------|-------|-------------|
| GET | `/staff/doctor/queue` (and `/staff/doctor/patients`) | Daily queue |
| GET | `/staff/doctor/consultation/{id}` | Consultation form |
| POST | `/staff/doctor/consultation/{id}` | Submit consultation + Rx |
| GET | `/staff/doctor/prescribe` | Prescriptions list |
| POST | `/staff/doctor/prescribe/{id}` | Issue prescription |

### Nurse
| Method | Route | Description |
|--------|-------|-------------|
| GET | `/staff/nurse/triage` | Triage queue (priority-ordered) |
| POST | `/staff/nurse/queue/triage/{id}` | Submit triage vitals |
| GET | `/staff/nurse/medications` | Medication administration |
| POST | `/staff/nurse/medications/{id}/administer` | Log dose administered |

### Receptionist
| Method | Route | Description |
|--------|-------|-------------|
| GET | `/staff/receptionist/reception` | Reception desk queue |
| POST | `/staff/receptionist/queue/check_in/{id}` | Check in patient |
| POST | `/staff/receptionist/queue/no_show/{id}` | Mark no-show |
| GET | `/staff/receptionist/billing` | Billing dashboard |
| POST | `/staff/receptionist/billing/checkout` | Process payment |

### Admin
| Method | Route | Description |
|--------|-------|-------------|
| GET | `/admin/dashboard` | Admin dashboard with staff counts |
| GET | `/admin/analytics` | Clinic analytics dashboard |
| GET | `/admin/security` | Audit log / security monitoring |
| GET / POST | `/admin/staff/onboard` | Staff onboarding |
| GET | `/admin/staff` | Staff directory |
| GET / POST | `/admin/staff/{id}/edit` | Edit staff |
| POST | `/admin/staff/{id}/delete` | Delete staff |
| GET | `/admin/support` | Support ticket management |
| POST | `/admin/support/reply` | Reply to a ticket |

---

## Application Workflow & State Machine

The patient journey is driven by the `appointment_status` ENUM:

```
scheduled ──check-in──► checked_in ──triage──► vitals_taken ──consultation──► completed
    │                        │
    ├──cancel──► cancelled   └──no-show──► no_show
                                         (bed transfer) ──► admitted
```

| Step | Role | Action |
|------|------|--------|
| 1. Book appointment | Patient | Reserve a 15-min slot (atomic, conflict-checked) |
| 2. Check-in | Receptionist | Assign queue number (advisory lock) + triage room |
| 3. Triage | Nurse | Record vitals → `vitals_taken`, assign consultation room |
| 4. Consultation | Doctor | Diagnose, write notes, issue prescriptions → `completed` |
| 5. Prescription | Doctor | Add medicines (priced into the bill) |
| 6. Medication admin | Nurse | Log administered doses |
| 7. Billing | Receptionist | Collect payment → bill `paid` |
| (branch) Admission | Doctor/Admin | Bed transfer → `admitted`; discharge adds admission fees |

---

## Setup & Installation

### Prerequisites
- **Rust** toolchain (stable) — install via [rustup](https://rustup.rs/)
- **PostgreSQL** running locally (or reachable via `DATABASE_URL`)
- A `.env` file at the project root (see [Configuration](#configuration))

### Database
Create a database that matches your `DATABASE_URL`, e.g.:

```bash
createdb patient_db
```

Migrations run automatically on startup — no manual migration step is required.

### Run

```bash
# Apply migrations, seed default accounts, and start the server
cargo run

# Or with auto-reload during development (requires cargo-watch)
cargo watch -x run
```

The server starts at **http://127.0.0.1:8080**. On first launch the schema is created and the five default accounts are seeded.

### Build for release

```bash
cargo build --release
./target/release/Web_Project
```

---

## Configuration

Create a `.env` file at the project root:

```env
# PostgreSQL connection string
DATABASE_URL=postgres://user:password@localhost/patient_db

# 64-byte secret used to sign session cookies
SESSION_SECRET=<a-64-byte-secret-string>
```

| Variable | Required | Description |
|----------|----------|-------------|
| `DATABASE_URL` | Yes | PostgreSQL connection string (the app panics on startup if unset) |
| `SESSION_SECRET` | Recommended | Key for signing session cookies; falls back to a built-in development key if unset |

> The connection pool is configured with a maximum of **5 connections** (`src/db/mod.rs`).

---

## Security Considerations

- **Password storage:** Argon2 hashing with per-password salts; hashes are never serialized into responses (`#[serde(skip_serializing)]`).
- **Session integrity:** signed cookie sessions; `SESSION_SECRET` should be a strong 64-byte value in production.
- **Access control:** every protected route is guarded server-side by role; unauthorized access yields `403`, unauthenticated access redirects to login.
- **User-enumeration resistance:** authentication failures and wrong-portal logins share a single generic error message.
- **Auditability:** authentication and account-management events are recorded in an append-only audit log.
- **XSS:** Tera escapes template output by default.
- **Referential safety:** cascade / set-null foreign keys keep the database consistent when records are removed.

---

## Development Notes & Known Limitations

- **Password reset tokens are stored in memory** (`Arc<Mutex<HashMap<...>>>`) and are therefore **cleared on server restart** — suitable for development/demo, not production.
- **Default development session key:** if `SESSION_SECRET` is unset, a static development key is used. Always set a real secret outside development.
- **Default seed password (`faipi`) is for demo/testing only** — change or remove the seeding routine before any real deployment.
- **Clinic hours** are fixed at 09:00–17:00 with 15-minute slots.
- **Email delivery is not integrated** — password-reset tokens are surfaced for the demo rather than emailed.
- The `target/` build directory and lock files are git-ignored (see `.gitignore`).

---

*Built with Rust 🦀 — Actix-Web · Tera · PostgreSQL · SQLx · Argon2*