use actix_web::{HttpResponse, Responder};

pub async fn support_form_page() -> impl Responder {
    HttpResponse::Ok().body("Support Form Placeholder")
}

pub async fn submit_support_ticket() -> impl Responder {
    HttpResponse::Ok().body("Ticket Submitted Placeholder")
}