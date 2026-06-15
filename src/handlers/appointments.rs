use actix_web::{web, HttpResponse, Responder};
use actix_session::Session;
use sqlx::PgPool;
use uuid::Uuid;
use tera::{Tera, Context};
use chrono::{NaiveDate, NaiveTime, Duration};
use serde::Deserialize;

use crate::models::appointment::{BookAppointmentForm, UIAppointmentSlot};
use crate::db::appointments::{get_doctor_busy_periods, get_patient_busy_periods, book_patient_appointment};

#[derive(Deserialize)]
pub struct BookingQuery {
    pub doctor_id: Option<Uuid>,
    pub date: Option<NaiveDate>,
    pub duration_minutes: Option<i64>, // Catch selected duration from URL parameters
}

#[derive(serde::Serialize)]
struct DoctorDropdownItem {
    id: Uuid,
    first_name: String,
    last_name: String,
}

pub async fn show_booking_form(
    tmpl: web::Data<Tera>,
    pool: web::Data<PgPool>,
    session: Session,
    query: web::Query<BookingQuery>,
) -> impl Responder {
    match session.get::<String>("role") {
        Ok(Some(role)) if role == "patient" => {},
        _ => return HttpResponse::SeeOther().append_header(("Location", "/patient/login")).finish(),
    };

    let patient_id = session.get::<Uuid>("user_id").unwrap_or_default().unwrap_or_default();

    let doctors = match sqlx::query_as!(
        DoctorDropdownItem,
        r#"
        SELECT s.id, s.first_name, s.last_name 
        FROM staff s
        JOIN users u ON s.id = u.id
        WHERE u.role = 'doctor'::user_role
        ORDER BY s.last_name ASC, s.first_name ASC
        "#
    )
    .fetch_all(pool.get_ref())
    .await {
        Ok(list) => list,
        Err(_) => Vec::new(),
    };

    let selected_doctor_id = query.doctor_id.or_else(|| doctors.first().map(|d| d.id));
    let selected_date = query.date.unwrap_or_else(|| chrono::Local::now().date_naive());
    let selected_duration = query.duration_minutes.unwrap_or(15); // Default to 15 mins standard layout

    let mut slots_grid = Vec::new();

    if let Some(doc_id) = selected_doctor_id {
        let doc_busy = get_doctor_busy_periods(pool.get_ref(), doc_id, selected_date).await.unwrap_or_default();
        let patient_busy = get_patient_busy_periods(pool.get_ref(), patient_id, selected_date).await.unwrap_or_default();

        // Clinic Operational Shift bounds: 09:00 AM to 05:00 PM
        let mut current_slot = NaiveTime::from_hms_opt(9, 0, 0).unwrap();
        let end_of_shift = NaiveTime::from_hms_opt(17, 0, 0).unwrap();

        // We step through the shift in 15-minute intervals
        while current_slot < end_of_shift {
            // But the end of *this* requested booking depends on the selected procedure type!
            let slot_end = current_slot + Duration::minutes(selected_duration);

            // A slot is invalid if its computed duration spills past the clinic closing time
            if slot_end > end_of_shift {
                break;
            }

            // Verify if the entire window needed for this procedure overlaps with any busy times
            let doc_conflict = doc_busy.iter().any(|(s, e)| current_slot < *e && slot_end > *s);
            let patient_conflict = patient_busy.iter().any(|(s, e)| current_slot < *e && slot_end > *s);

            slots_grid.push(UIAppointmentSlot {
                time_string: current_slot.format("%I:%M %p").to_string(),
                raw_time: current_slot,
                is_available: !doc_conflict && !patient_conflict,
            });

            // Keep the grid increments regular at 15 minutes so patients can see all options
            current_slot = current_slot + Duration::minutes(15);
        }
    }

    let mut ctx = Context::new();
    ctx.insert("doctors", &doctors);
    ctx.insert("slots", &slots_grid);
    ctx.insert("selected_doctor_id", &selected_doctor_id);
    ctx.insert("selected_date", &selected_date.to_string());
    ctx.insert("selected_duration", &selected_duration);
    
    let email = session.get::<String>("email").unwrap_or_default().unwrap_or_default();
    ctx.insert("email", &email);

    match tmpl.render("patient/book_appointment.html", &ctx) {
        Ok(html) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
        Err(e) => HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
    }
}

// Struct to explicitly read duration_minutes from the POST body
#[derive(Deserialize)]
pub struct SubmitAppointmentForm {
    pub doctor_id: Uuid,
    pub date: NaiveDate,
    pub start_time: NaiveTime,
    pub duration_minutes: i64,
}

pub async fn submit_appointment(
    pool: web::Data<PgPool>,
    session: Session,
    form: web::Form<SubmitAppointmentForm>,
) -> impl Responder {
    let patient_id = match session.get::<Uuid>("user_id") {
        Ok(Some(id)) => id,
        _ => return HttpResponse::SeeOther().append_header(("Location", "/patient/login")).finish(),
    };

    let computed_end_time = form.start_time + Duration::minutes(form.duration_minutes);

    // Call database booking engine using the specific calculated layout boundary
    match crate::db::appointments::book_patient_appointment(
        &pool, 
        patient_id, 
        form.doctor_id, 
        form.date, 
        form.start_time,
        computed_end_time
    ).await {
        Ok(_) => {
            HttpResponse::SeeOther()
                .append_header(("Location", "/patient/dashboard?success=booked"))
                .finish()
        }
        Err(e) => HttpResponse::BadRequest().body(format!("Scheduling failed: {}", e)),
    }
}

#[derive(Deserialize)]
pub struct QueueFilterParams {
    pub view: Option<String>,
}

/// GET route handler presenting the doctor with a dynamically filtered clinical queue tracking layout.
pub async fn doctor_daily_queue_page(
    pool: web::Data<PgPool>,
    session: Session,
    tmpl: web::Data<Tera>,
    query: web::Query<QueueFilterParams>,
) -> impl Responder {
    // Role Enforcement Gatekeeper
    match session.get::<String>("role") {
        Ok(Some(role)) if role == "doctor" => {},
        _ => return HttpResponse::SeeOther().append_header(("Location", "/staff/login")).finish(),
    };

    let doctor_id = match session.get::<Uuid>("user_id") {
        Ok(Some(id)) => id,
        _ => return HttpResponse::SeeOther().append_header(("Location", "/staff/login")).finish(),
    };

    // Determine filter mode (defaults to "today" if parameter is missing or unrecognized)
    let filter_mode = match query.view.as_deref() {
        Some("all") => "all",
        _ => "today",
    };

    let today = chrono::Local::now().date_naive();
    let email = session.get::<String>("email").unwrap_or_default().unwrap_or_default();

    // Query our dynamically filtered clinical dataset slice
    let appointments = match crate::db::appointments::get_doctor_daily_appointments(&pool, doctor_id, filter_mode).await {
        Ok(data) => data,
        Err(err) => return HttpResponse::InternalServerError().body(format!("Clinical query tracking failure: {}", err)),
    };

    let mut ctx = Context::new();
    ctx.insert("email", &email);
    ctx.insert("formatted_date", &today.format("%A, %B %d, %Y").to_string());
    ctx.insert("queue", &appointments);
    ctx.insert("queue_count", &appointments.len());
    ctx.insert("current_view", filter_mode); // Pass view to template to toggle active tab states

    // UPDATED: Now looks inside templates/staff/doctor/ folder layout
    match tmpl.render("staff/doctor/doctor_queue.html", &ctx) {
        Ok(html) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
        Err(e) => HttpResponse::InternalServerError().body(format!("Rendering compilation error: {}", e)),
    }
}

/// Renders the Receptionist Dashboard
pub async fn reception_desk_page(
    pool: web::Data<sqlx::PgPool>,
    session: actix_session::Session,
    tmpl: web::Data<tera::Tera>,
) -> impl actix_web::Responder {
    // Only allow admins or receptionists
    match session.get::<String>("role") {
        Ok(Some(role)) if role == "receptionist" || role == "admin" => {},
        _ => return actix_web::HttpResponse::SeeOther().append_header(("Location", "/staff/login")).finish(),
    };

    let schedule = crate::db::appointments::get_today_clinic_schedule(&pool).await.unwrap_or_default();
    
    let mut ctx = tera::Context::new();
    ctx.insert("schedule", &schedule);
    ctx.insert("date", &chrono::Local::now().format("%A, %B %d, %Y").to_string());

    ctx.insert("specific_role", &session.get::<String>("role").unwrap_or_default().unwrap_or_default());

    // UPDATED: Now looks inside templates/staff/receptionist/ folder layout
    match tmpl.render("staff/receptionist/reception.html", &ctx) {
        Ok(html) => actix_web::HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
        Err(e) => actix_web::HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
    }
}

/// POST route to check a patient in, assign a queue number, and refresh the page
pub async fn process_check_in(
    pool: web::Data<sqlx::PgPool>,
    session: actix_session::Session,
    path: web::Path<uuid::Uuid>, 
) -> impl actix_web::Responder {
    
    match session.get::<String>("role") {
        Ok(Some(role)) if role == "receptionist" || role == "admin" => {}, 
        _ => return actix_web::HttpResponse::SeeOther().append_header(("Location", "/staff/login")).finish(),
    };

    let appointment_id = path.into_inner();

    match crate::db::appointments::check_in_patient(&pool, appointment_id).await {
        Ok(_) => {
            // Redirect back to the Receptionist dashboard so the page refreshes automatically!
            actix_web::HttpResponse::SeeOther()
                .append_header(("Location", "/staff/receptionist/reception?success=checked_in"))
                .finish()
        }
        Err(_) => {
            actix_web::HttpResponse::SeeOther()
                .append_header(("Location", "/staff/receptionist/reception?error=check_in_failed"))
                .finish()
        }
    }
}