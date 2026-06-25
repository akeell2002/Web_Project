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
    pub visit_type:       Option<String>,
}

#[derive(serde::Serialize)]
struct DoctorDropdownItem {
    id:         Uuid,
    first_name: String,
    last_name:  String,
}

/// GET - patient appointment booking form with time-slot grid
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
    let selected_visit_type = query.visit_type.clone().unwrap_or_default();

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

        let mut current_slot = NaiveTime::from_hms_opt(9, 0, 0).unwrap();
        let end_of_shift     = NaiveTime::from_hms_opt(17, 0, 0).unwrap();

        while current_slot < end_of_shift {
            let slot_end = current_slot + Duration::minutes(selected_duration);
            if slot_end > end_of_shift { break; }

            let doc_conflict     = doc_busy.iter().any(|(s, e)| current_slot < *e && slot_end > *s);
            let patient_conflict = patient_busy.iter().any(|(s, e)| current_slot < *e && slot_end > *s);

            slots_grid.push(UIAppointmentSlot {
                time_string:  current_slot.format("%I:%M %p").to_string(),
                raw_time:     current_slot,
                is_available: !doc_conflict && !patient_conflict,
            });

            current_slot = current_slot + Duration::minutes(15);
        }
    }

    let email        = session.get::<String>("email").unwrap_or_default().unwrap_or_default();
    let display_name = crate::handlers::get_display_name(&session);

    let mut ctx = Context::new();
    ctx.insert("specific_role",       "patient");
    ctx.insert("email",               &email);
    ctx.insert("display_name",        &display_name);
    ctx.insert("doctors",             &doctors);
    ctx.insert("slots",               &slots_grid);
    ctx.insert("selected_doctor_id",  &selected_doctor_id);
    ctx.insert("selected_date",       &selected_date.to_string());
    ctx.insert("selected_duration",   &selected_duration);
    ctx.insert("selected_visit_type", &selected_visit_type);

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
    pub visit_type:       String,
}

/// POST - book the appointment
pub async fn submit_appointment(
    pool:    web::Data<PgPool>,
    session: Session,
    form:    web::Form<SubmitAppointmentForm>,
) -> impl Responder {
    let patient_id = match session.get::<Uuid>("user_id") {
        Ok(Some(id)) => id,
        _ => return HttpResponse::SeeOther().append_header(("Location", "/patient/login")).finish(),
    };
    match session.get::<String>("role") {
        Ok(Some(role)) if role == "patient" => {}
        _ => return HttpResponse::SeeOther().append_header(("Location", "/patient/login")).finish(),
    }

    let end_time = form.start_time + Duration::minutes(form.duration_minutes);
    let priority = match form.visit_type.as_str() {
        "emergency"  => 1,
        "specialist" => 2,
        "sick_visit" => 3,
        "procedure"  => 2,
        _            => 4,
    };

    match crate::db::appointments::book_patient_appointment(
        &pool, patient_id, form.doctor_id, form.date,
        form.start_time, end_time, priority,
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

// --- RECEPTIONIST DESK ---

/// GET - receptionist reception desk
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
    let display_name = crate::handlers::get_display_name(&session);

    let mut ctx = Context::new();
    ctx.insert("specific_role", &current_role);
    ctx.insert("email",         &email);
    ctx.insert("display_name",  &display_name);
    ctx.insert("schedule",      &schedule);
    ctx.insert("date",          &chrono::Local::now().format("%A, %B %d, %Y").to_string());

    match tmpl.render("staff/receptionist/reception.html", &ctx) {
        Ok(html) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
        Err(e)   => HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
    }
}

/// POST - check a patient in from the reception desk
pub async fn process_check_in(
    pool:    web::Data<PgPool>,
    session: Session,
    path:    web::Path<Uuid>,
) -> impl Responder {
    match session.get::<String>("role") {
        Ok(Some(role)) if role == "receptionist" || role == "admin" => {}
        _ => return HttpResponse::Forbidden().body("Access Denied: Receptionist access required."),
    }

    let appointment_id = path.into_inner();

    match crate::db::appointments::check_in_patient(&pool, appointment_id).await {
        Ok(_)  => {
            crate::handlers::audit_clinical_action(&pool, &session, appointment_id, "patient_checked_in", "checked in").await;
            HttpResponse::SeeOther()
                .append_header(("Location", "/staff/receptionist/reception?success=checked_in"))
                .finish()
        }
        Err(_) => HttpResponse::SeeOther()
            .append_header(("Location", "/staff/receptionist/reception?error=check_in_failed"))
            .finish(),
    }
}

/// POST - receptionist marks an appointment as no-show
pub async fn process_no_show(
    pool:    web::Data<PgPool>,
    session: Session,
    path:    web::Path<Uuid>,
) -> impl Responder {
    match session.get::<String>("role") {
        Ok(Some(role)) if role == "receptionist" || role == "admin" => {}
        _ => return HttpResponse::Forbidden().body("Access Denied: Receptionist access required."),
    }

    let appointment_id = path.into_inner();

    match crate::db::appointments::mark_appointment_no_show(&pool, appointment_id).await {
        Ok(_)  => HttpResponse::SeeOther()
            .append_header(("Location", "/staff/receptionist/reception?success=no_show"))
            .finish(),
        Err(_) => HttpResponse::SeeOther()
            .append_header(("Location", "/staff/receptionist/reception?error=no_show_failed"))
            .finish(),
    }
}

// --- UPDATE APPOINTMENT ---

#[derive(Deserialize)]
pub struct UpdateQuery {
    pub doctor_id:        Option<Uuid>,
    pub date:             Option<NaiveDate>,
    pub duration_minutes: Option<i64>,
    pub visit_type:       Option<String>,
}

/// GET - patient appointment update form (pre-seeded with current values)
pub async fn show_update_form(
    tmpl:    web::Data<Tera>,
    pool:    web::Data<PgPool>,
    session: Session,
    path:    web::Path<Uuid>,
    query:   web::Query<UpdateQuery>,
) -> impl Responder {
    let patient_id = match session.get::<Uuid>("user_id") {
        Ok(Some(id)) => id,
        _ => return HttpResponse::SeeOther().append_header(("Location", "/patient/login")).finish(),
    };
    match session.get::<String>("role") {
        Ok(Some(role)) if role == "patient" => {}
        _ => return HttpResponse::SeeOther().append_header(("Location", "/patient/login")).finish(),
    }

    let appointment_id   = path.into_inner();
    let email            = session.get::<String>("email").unwrap_or_default().unwrap_or_default();
    let display_name     = crate::handlers::get_display_name(&session);
    let selected_visit_type = query.visit_type.clone().unwrap_or_default();

    let original = match crate::db::appointments::get_patient_appointment_by_id(&pool, appointment_id, patient_id).await {
        Ok(Some(a)) => a,
        Ok(None)    => return HttpResponse::SeeOther().append_header(("Location", "/patient/dashboard?error=not_found")).finish(),
        Err(e)      => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };

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

    let selected_doctor_id = query.doctor_id.unwrap_or(original.doctor_id.unwrap_or_default());
    let selected_date      = query.date.unwrap_or(original.date);
    let selected_duration  = query.duration_minutes.unwrap_or(15);

    let doc_busy     = get_doctor_busy_periods(pool.get_ref(), selected_doctor_id, selected_date).await.unwrap_or_default();
    let patient_busy = get_patient_busy_periods(pool.get_ref(), patient_id, selected_date).await.unwrap_or_default();

    let mut slots_grid = Vec::new();
    let mut current_slot = NaiveTime::from_hms_opt(9, 0, 0).unwrap();
    let end_of_shift     = NaiveTime::from_hms_opt(17, 0, 0).unwrap();

    while current_slot < end_of_shift {
        let slot_end = current_slot + Duration::minutes(selected_duration);
        if slot_end > end_of_shift { break; }

        let is_own_slot  = current_slot == original.start_time
                           && selected_date == original.date
                           && selected_doctor_id == original.doctor_id.unwrap_or_default();
        let doc_conflict = !is_own_slot && doc_busy.iter().any(|(s, e)| current_slot < *e && slot_end > *s);
        let pat_conflict = !is_own_slot && patient_busy.iter().any(|(s, e)| current_slot < *e && slot_end > *s);

        slots_grid.push(UIAppointmentSlot {
            time_string:  current_slot.format("%I:%M %p").to_string(),
            raw_time:     current_slot,
            is_available: !doc_conflict && !pat_conflict,
        });

        current_slot = current_slot + Duration::minutes(15);
    }

    let mut ctx = Context::new();
    ctx.insert("specific_role",       "patient");
    ctx.insert("email",               &email);
    ctx.insert("display_name",        &display_name);
    ctx.insert("appointment_id",      &appointment_id);
    ctx.insert("doctors",             &doctors);
    ctx.insert("slots",               &slots_grid);
    ctx.insert("selected_doctor_id",  &selected_doctor_id);
    ctx.insert("selected_date",       &selected_date.to_string());
    ctx.insert("selected_duration",   &selected_duration);
    ctx.insert("selected_visit_type", &selected_visit_type);
    ctx.insert("original_start_time", &original.start_time);

    match tmpl.render("patient/update_appointment.html", &ctx) {
        Ok(html) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
        Err(e)   => HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
    }
}

#[derive(Deserialize)]
pub struct SubmitUpdateForm {
    pub doctor_id:        Uuid,
    pub date:             NaiveDate,
    pub start_time:       NaiveTime,
    pub duration_minutes: i64,
    pub visit_type:       String,
}

/// POST - save rescheduled appointment
pub async fn submit_update_appointment(
    pool:    web::Data<PgPool>,
    session: Session,
    path:    web::Path<Uuid>,
    form:    web::Form<SubmitUpdateForm>,
) -> impl Responder {
    let patient_id = match session.get::<Uuid>("user_id") {
        Ok(Some(id)) => id,
        _ => return HttpResponse::SeeOther().append_header(("Location", "/patient/login")).finish(),
    };

    let appointment_id = path.into_inner();
    let end_time       = form.start_time + Duration::minutes(form.duration_minutes);
    let priority = match form.visit_type.as_str() {
        "emergency"  => 1,
        "specialist" => 2,
        "sick_visit" => 3,
        "procedure"  => 2,
        _            => 4,
    };

    match crate::db::appointments::update_patient_appointment(
        &pool, appointment_id, patient_id,
        form.doctor_id, form.date, form.start_time, end_time, priority,
    ).await {
        Ok(_) => HttpResponse::SeeOther()
            .append_header(("Location", "/patient/dashboard?success=appointment_updated"))
            .finish(),
        Err(e) => HttpResponse::BadRequest().body(format!("Failed to reschedule: {}", e)),
    }
}

/// POST - cancel a patient's own appointment
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
        _ => return HttpResponse::SeeOther().append_header(("Location", "/patient/login")).finish(),
    }

    let appointment_id = path.into_inner();

    match crate::db::appointments::cancel_patient_appointment(&pool, appointment_id, patient_id).await {
        Ok(_) => HttpResponse::SeeOther()
            .append_header(("Location", "/patient/dashboard?success=appointment_cancelled"))
            .finish(),
        Err(e) => HttpResponse::InternalServerError().body(format!("Failed to cancel: {}", e)),
    }
}
