### Group 02 Web Programming Assignmment

## Patient Management System
A full-stack hospital management web application built with Rust, Actix-Web, Tera, PostgreSQL, and Bootstrap. The system simulates a real-world clinical workflow, allowing patients and healthcare staff to manage appointments, consultations, billing, admissions, and medical records through dedicated role-based portals.

The application was designed to model realistic healthcare operations while demonstrating secure authentication, role-based access control, concurrency-safe scheduling, audit logging, and enterprise-grade backend architecture.

## Technology Stack
Backend - Rust + Actix-Web + Tera Templates + SQLx
Database - PostgreSQL
Frontend - HTML5 + CSS3 + JavaScript + Bootstrap 5.3
Auth & Security - Argon2 Password Hashing
Session Management - actix-session

## Project Structure
. 
├── migrations/
├── src/
    ├── db/
        └── appointments/
    ├── handlers/
        └── admin/
        └── appointments/
        └── auth/
    └── models/
    └── utils.rs
    └── main.rs
├── static/
    ├── app.js
    ├── logo.png
    └── styles.css
├── templates/
        ├── admin/
        ├── auth/
        ├── patient/
        ├── shared/
        ├── staff/
            └── doctor/
            └── nurse/
            └── receptionist/
        └── support/
        └── index.html
├── .env
├── Cargo.toml
└── README.md

## Installation & Setup
# Prerequisites:
- Rust (1.95.0 or later)
- PostgreSQL + PgAdmin4 + sqlx-cli (latest stable version)
- Cargo (comes with Rust installation)

# Steps:
1. Download the project repository.
2. Navigate to the project directory.
3. Edit the `.env` file in the root directory and configure your database connection string as:
    `DATABASE_URL=postgresql://postgres:<password>@localhost/patient_management`
4. Open PGAdmin or any PostgreSQL client and create a new database named `patient_management` at localhost. 
   Make sure the username and password match the connection string in the `.env` file.
5. Run the following command to apply database migrations:
    1. `cargo install sqlx-cli --no-default-features --features postgres`
    2. `sqlx migrate run`
6. Start the application with:
    `cargo run`
    or 
    `cargo watch -x run` (install cargo-watch if not installed)
7. The application will start at: http://127.0.0.1:8080

## Default Accounts
The application automatically seeds default users for demonstration purposes.
These accounts are intended for development and demonstration only.
All seeded accounts can be accessed with the password `faipi`.

| Role         | Username                        |
|--------------|---------------------------------|
| Admin        | admin@clinic.com                |
| Doctor       | doctor@clinic.com               | 
| Nurse        | nurse@clinic.com                | 
| Receptionist | receptionist@clinic.com         |
| Patient      | patient@clinic.com              |

## Features
# Patient Portal
- Register and log in
- Forget password feature
- View, book, edit and cancel appointments
- View medical history and past visits
- View billing records
- Manage personal profile and update particulars

# Receptionist Portal
- Register patients
- Check-in & manage appointments
- Manage billing
- View patient directory and medical history
- Edit patient records and update particulars
- Generate medical report of a patient
- View Bed availability and request transfers
- Manage personal profile and update particulars

# Nurse Portal
- Record patient vitals
- Manage triage queue
- Administer medications
- View patient directory and medical history
- Edit patient records and update particulars
- Generate medical report of a patient
- View Bed availability and request transfers
- Manage personal profile and update particulars

# Doctor Portal
- View consultation queue
- Record diagnoses
- Prescribe medication
- Manage admission and discharge of patients
- View patient directory and medical history
- Edit patient records and update particulars
- Generate medical report of a patient
- View Bed availability and approve transfers
- Manage personal profile and update particulars

# Admin Portal
- Manage staff accounts (add, edit, or remove staff)
- View analytics dashboard (appointments, billing, and patient statistics)
- Review audit logs (user activity and administrative actions)
- Handle support tickets (view, respond, and resolve with timestamps)
- View patient directory and able to delete patient records

## Technical Highlights
- Role-Based Access Control
- Protected Routes
- Portal Segmentation (Patient & Staff)
- 2 Factor Authentication (2FA)
- Secure password hashing using Argon2
- Secure session management with actix-session
- Password reset via email (cli version)
- Concurrency-Safe appointment booking engine
- PostgreSQL advisory locks queue management
- Dynamic Triage Prioritization Queue
- Account activity audit logging
- Administrative action monitoring
- Support ticketing system
- Staff and Patient Management
- Bed Tracking and Management
- Billing and Invoicing
- Medical Report Generation and Historical Record Keeping
- Analytics Dashboard

## Future Improvements
- Email notifications
- Docker deployment