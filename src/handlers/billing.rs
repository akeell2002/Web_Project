use actix_web::{web, HttpResponse, Responder};
use actix_session::Session;
use sqlx::PgPool;
use tera::{Context, Tera};
use uuid::Uuid;

use crate::db::billing::{get_unpaid_bills, mark_bill_as_paid};
use crate::models::billing::ProcessPaymentForm;

/// GET: Renders a dashboard containing all active unpaid hospital invoices
pub async fn show_billing_dashboard(
    session: Session,
    tmpl: web::Data<Tera>,
    pool: web::Data<PgPool>,
) -> impl Responder {
    // Role Authorization Check Guard
    match session.get::<String>("role") {
        Ok(Some(role)) if role == "receptionist" || role == "admin" => {},
        _ => return HttpResponse::SeeOther().insert_header(("Location", "/staff/login")).finish(),
    };

    let current_role = session.get::<String>("role").unwrap_or_default().unwrap_or_default();
    let email = session.get::<String>("email").unwrap_or_default().unwrap_or_default();
    let staff_name = email.split('@').next().unwrap_or("Staff").to_string();

    // Query pending invoices
    match get_unpaid_bills(&pool).await {
        Ok(unpaid_items) => {
            let mut ctx = Context::new();
            ctx.insert("specific_role", &current_role);
            ctx.insert("email", &email);
            ctx.insert("staff_name", &staff_name);
            ctx.insert("bills", &unpaid_items);

            match tmpl.render("staff/receptionist/billing_dashboard.html", &ctx) {
                Ok(html) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
                Err(e) => {
                    eprintln!("Template Error: {}", e);
                    HttpResponse::InternalServerError().body("Template compilation failure.")
                }
            }
        }
        Err(e) => {
            eprintln!("DB Retrieval Error: {}", e);
            HttpResponse::InternalServerError().body("Failed to gather pending invoices.")
        }
    }
}

/// POST: Receives a targeted invoice payment processing form submission request
pub async fn checkout_bill_submit(
    session: Session,
    pool: web::Data<PgPool>,
    form: web::Form<ProcessPaymentForm>,
) -> impl Responder {
    // Validate role context permissions
    match session.get::<String>("role") {
        Ok(Some(role)) if role == "receptionist" || role == "admin" => {},
        _ => return HttpResponse::Forbidden().body("Access Denied: Specialized personnel required."),
    };

    // Safely parse out who is processing this checkout event 
    let staff_user_id = match session.get::<Uuid>("user_id") {
        Ok(Some(id)) => id,
        _ => return HttpResponse::Unauthorized().body("Session validation issue."),
    };

    match mark_bill_as_paid(&pool, form.bill_id, staff_user_id).await {
        Ok(_) => {
            // Successfully collected! Refresh the screen tracking directory grid
            HttpResponse::SeeOther()
                .insert_header(("Location", "/staff/receptionist/billing"))
                .finish()
        }
        Err(e) => {
            eprintln!("Payment Settlement crash error: {}", e);
            HttpResponse::InternalServerError().body("Failed to clear billing balance changes state.")
        }
    }
}