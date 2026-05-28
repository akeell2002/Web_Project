# Patient Management System

A modern SSR web application built with Rust, Actix Web, Tera, and PostgreSQL. The project is structured as a layered enterprise backend with clear separation between HTTP handlers, database access, domain models, utility code, and frontend templates.

## ADMIN ACCOUNT
admin@clinic.com
Password123!

## Project Goals

This project demonstrates:

- Rust backend development with `struct` and `impl` based design
- Modular architecture with controller, service, persistence, and view layers
- Server Side Rendering with reusable HTML templates
- Authentication and session-based authorization
- CRUD workflows backed by a relational database
- Form validation and business-rule checks
- Responsive, server-rendered UI for enterprise-style workflows

## Current Feature Set

The repository currently includes:

- User registration and login
- Session-based authentication with role storage
- Patient registration, listing, detail view, edit, and delete flows
- PostgreSQL persistence via SQLx
- Tera templates for SSR pages
- Static CSS assets for the frontend

The database schema is also prepared for future enterprise modules such as appointments and medical records.

## Architecture

The codebase follows a layered layout:

- `src/main.rs` bootstraps Actix Web, sessions, templates, and routing
- `src/handlers/` contains HTTP request handlers for auth and patients
- `src/db/` contains SQLx database operations
- `src/models/` contains shared application data structures and forms
- `src/utils.rs` contains supporting logic such as password hashing
- `templates/` contains SSR HTML pages rendered by Tera
- `static/` contains CSS and other frontend assets

This design keeps presentation logic out of persistence code and makes the system easier to extend with additional business modules.

## Domain Model

The schema supports a realistic hospital or clinic workflow:

- `users` stores staff and patient accounts with roles such as `admin`, `doctor`, `nurse`, `receptionist`, and `patient`
- `patients` stores demographic and contact information
- `appointments` stores visit scheduling data
- `medical_records` stores diagnoses, prescriptions, and notes

That structure allows the project to grow into a fuller enterprise system without changing the underlying data model.

## Technology Stack

- Rust 2024 edition
- Actix Web for HTTP routing and middleware
- Actix Session for cookie-based session management
- SQLx for PostgreSQL access
- Tera for server-side HTML rendering
- Argon2 for password hashing
- dotenv for local configuration
- env_logger and tracing for runtime logging

## Setup

### 1. Prerequisites

- Rust toolchain
- PostgreSQL
- A `.env` file with `DATABASE_URL` configured

### 2. Database

Run the migration to create the schema:

```bash
cargo sqlx migrate run
```

If you prefer to inspect the schema first, see [migrations/01_schema_init.sql](migrations/01_schema_init.sql).

### 3. Run the app

```bash
cargo run
```

The app starts on `http://127.0.0.1:8080`.

## Routes

- `/` renders the login page
- `/login` handles authentication
- `/register` handles account creation
- `/dashboard` renders the authenticated landing page
- `/patients` shows the patient list
- `/patients/add` creates a new patient
- `/patients/{id}` shows a patient profile
- `/patients/{id}/edit` updates a patient
- `/patients/{id}/delete` removes a patient

## Database Schema

The schema is defined in [migrations/01_schema_init.sql](migrations/01_schema_init.sql) and includes:

- `users`
- `patients`
- `appointments`
- `medical_records`

This supports both the current patient-management workflow and the planned expansion into broader clinical operations.

## Implemented Enterprise Modules

- Appointment queue management
- Patient history timeline
- Staff role-based access control per module
- Medical report generation
- Billing and invoice workflows
