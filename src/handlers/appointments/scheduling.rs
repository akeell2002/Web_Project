use actix_web::{web, HttpResponse, Responder};
use actix_session::Session;
use sqlx::PgPool;
use uuid::Uuid;
use tera::{Tera, Context};
use chrono::{NaiveDate, NaiveTime, Duration};
use serde::Deserialize;

use crate::models::appointment::UIAppointmentSlot;
use crate::db::appointments::{get_doctor_busy_periods, get_patient_busy_periods};

#[derive(Deserialize)]
pub struct BookingQuery {
    pub doctor_id:        Option<Uuid>,
    pub date:             Option<NaiveDate>,
    pub duration_minutes: Option<i64>,
}

#[derive(serde::Serialize)]
struct DoctorDropdownItem {
    id:         Uuid,
    first_name: String,
    last_name:  String,
}

/// GET — patient appointment booking form with time-slot grid
pub async fn show_booking_form(
    tmpl:    web::Data<Tera>,
    pool:    web::Data<PgPool>,
    session: Session,
    query:   web::Query<BookingQuery>,
) -> impl Responder {
    match session.get::<String>("role") {
        Ok(Some(role)) if role == "patient" => {}
        _ => return HttpResponse::SeeOther().append_header(("Location", "/patient/login")).finish(),
    }

    let patient_id = session.get::<Uuid>("user_id").unwrap_or_default().unwrap_or_default();

    let doctors = sqlx::query_as!(
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
    .await
    .unwrap_or_default();

    let selected_doctor_id = query.doctor_id.or_else(|| doctors.first().map(|d| d.id));
    let selected_date      = query.date.unwrap_or_else(|| chrono::Local::now().date_naive());
    let selected_duration  = query.duration_minutes.unwrap_or(15);

    let mut slots_grid = Vec::new();

    if let Some(doc_id) = selected_doctor_id {
        let doc_busy     = get_doctor_busy_periods(pool.get_ref(), doc_id, selected_date).await.unwrap_or_default();
        let patient_busy = get_patient_busy_periods(pool.get_ref(), patient_id, selected_date).await.unwrap_or_default();

        let mut current_slot  = NaiveTime::from_hms_opt(9, 0, 0).unwrap();
        let end_of_shift      = NaiveTime::from_hms_opt(17, 0, 0).unwrap();

        while current_slot < end_of_shift {
            let slot_end = current_slot + Duration::minutes(selected_duration);
            if slot_end > end_of_shift { break; }

            let doc_conflict     = doc_busy.iter().any(|(s, e)| current_slot < *e && slot_end > *s);
            let patient_conflict = patient_busy.iter().any(|(s, e)| current_slot < *e && slot_end > *s);

            slots_grid.push(UIAppointmentSlot {
                time_string: current_slot.format("%I:%M %p").to_string(),
                raw_time:    current_slot,
                is_available: !doc_conflict && !patient_conflict,
            });

            current_slot = current_slot + Duration::minutes(15);
        }
    }

    let email      = session.get::<String>("email").unwrap_or_default().unwrap_or_default();
    let staff_name = email.split('@').next().unwrap_or("Patient").to_string();

    let mut ctx = Context::new();
    ctx.insert("specific_role",       "patient");
    ctx.insert("email",               &email);
    ctx.insert("staff_name",          &staff_name);
    ctx.insert("doctors",             &doctors);
    ctx.insert("slots",               &slots_grid);
    ctx.insert("selected_doctor_id",  &selected_doctor_id);
    ctx.insert("selected_date",       &selected_date.to_string());
    ctx.insert("selected_duration",   &selected_duration);

    match tmpl.render("patient/book_appointment.html", &ctx) {
        Ok(html) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
        Err(e)   => HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
    }
}

#[derive(Deserialize)]
pub struct SubmitAppointmentForm {
    pub doctor_id:        Uuid,
    pub date:             NaiveDate,
    pub start_time:       NaiveTime,
    pub duration_minutes: i64,
}

/// POST — book the appointment
pub async fn submit_appointment(
    pool:    web::Data<PgPool>,
    session: Session,
    form:    web::Form<SubmitAppointmentForm>,
) -> impl Responder {
    let patient_id = match session.get::<Uuid>("user_id") {
        Ok(Some(id)) => id,
        _ => return HttpResponse::SeeOther().append_header(("Location", "/patient/login")).finish(),
    };

    let end_time = form.start_time + Duration::minutes(form.duration_minutes);

    match crate::db::appointments::book_patient_appointment(
        &pool, patient_id, form.doctor_id, form.date, form.start_time, end_time,
    )
    .await
    {
        Ok(appt) => {
            let email = session.get::<String>("email").unwrap_or_default().unwrap_or_default();
            eprintln!(
                "\n[MOCK EMAIL] To: {} | Appointment {} confirmed on {} at {}\n",
                email, appt.id, form.date, form.start_time.format("%I:%M %p")
            );
            HttpResponse::SeeOther()
                .append_header(("Location", "/patient/dashboard?success=booked"))
                .finish()
        }
        Err(e) => HttpResponse::BadRequest().body(format!("Scheduling failed: {}", e)),
    }
}

/// GET — receptionist reception desk
pub async fn reception_desk_page(
    pool:    web::Data<PgPool>,
    session: Session,
    tmpl:    web::Data<Tera>,
) -> impl Responder {
    match session.get::<String>("role") {
        Ok(Some(role)) if role == "receptionist" || role == "admin" => {}
        _ => return HttpResponse::SeeOther().append_header(("Location", "/staff/login")).finish(),
    }

    let schedule     = crate::db::appointments::get_today_clinic_schedule(&pool).await.unwrap_or_default();
    let current_role = session.get::<String>("role").unwrap_or_default().unwrap_or_default();
    let email        = session.get::<String>("email").unwrap_or_default().unwrap_or_default();
    let staff_name   = email.split('@').next().unwrap_or("Staff").to_string();

    let mut ctx = Context::new();
    ctx.insert("specific_role", &current_role);
    ctx.insert("email",         &email);
    ctx.insert("staff_name",    &staff_name);
    ctx.insert("schedule",      &schedule);
    ctx.insert("date",          &chrono::Local::now().format("%A, %B %d, %Y").to_string());

    match tmpl.render("staff/receptionist/reception.html", &ctx) {
        Ok(html) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
        Err(e)   => HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
    }
}

/// POST — check a patient in from the reception desk
pub async fn process_check_in(
    pool:    web::Data<PgPool>,
    session: Session,
    path:    web::Path<Uuid>,
) -> impl Responder {
    match session.get::<String>("role") {
        Ok(Some(role)) if role == "receptionist" || role == "admin" => {}
        _ => return HttpResponse::SeeOther().append_header(("Location", "/staff/login")).finish(),
    }

    let appointment_id = path.into_inner();

    match crate::db::appointments::check_in_patient(&pool, appointment_id).await {
        Ok(_) => HttpResponse::SeeOther()
            .append_header(("Location", "/staff/receptionist/reception?success=checked_in"))
            .finish(),
        Err(_) => HttpResponse::SeeOther()
            .append_header(("Location", "/staff/receptionist/reception?error=check_in_failed"))
            .finish(),
    }
}

/// POST — patient cancels their own appointment
pub async fn cancel_appointment(
    pool:    web::Data<PgPool>,
    session: Session,
    path:    web::Path<Uuid>,
) -> impl Responder {
    let patient_id = match session.get::<Uuid>("user_id") {
        Ok(Some(id)) => id,
        _ => return HttpResponse::SeeOther().append_header(("Location", "/patient/login")).finish(),
    };

    match session.get::<String>("role") {
        Ok(Some(role)) if role == "patient" => {}
        _ => return HttpResponse::Forbidden().body("Only patients can cancel their own appointments."),
    }

    let appointment_id = path.into_inner();

    match crate::db::appointments::cancel_patient_appointment(&pool, appointment_id, patient_id).await {
        Ok(_)  => HttpResponse::SeeOther().append_header(("Location", "/patient/dashboard?success=cancelled")).finish(),
        Err(_) => HttpResponse::SeeOther().append_header(("Location", "/patient/dashboard?error=cancel_failed")).finish(),
    }
}
