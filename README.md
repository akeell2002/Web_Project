Cargo.toml: The project manifest. It specifies package metadata and manages every external library crate your team needs—such as actix-web, template tools, and database components.

.gitignore: Instructs Git which system generation files (like the heavy /target build output folder) to exclude from your cloud repository so you do not push unnecessary binary tracking files.

templates/: This folder lives outside of src because template engines expect it at the project root. This is your View layer, hosting all raw HTML layouts for Server-Side Rendering (SSR).

main.rs: The main execution runtime engine. It loads application modules, configures environment settings, establishes database connections, and builds the HTTP server platform to handle client web traffic.

db/: Handles data persistence layers. This is where database connection pools are defined and raw SQL execution statements are structured.

models/: Handles database mapping and structure definition. This directory holds your Rust data objects (structs) and behavioral contracts (traits) representing database entities like Patient, Doctor, and Appointment.

handlers/: Represents your Controller layer. These endpoints extract incoming data from client browser requests, call the correct background business workflows, and compile data directly into your HTML layouts.

services/: Houses your domain's logical control functions. This layer implements core business rules separate from raw web actions, such as running calculations or double-checking schedule availability before updating database records.


download the extension SQLITE VIEWER