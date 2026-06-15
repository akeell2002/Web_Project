use actix_web::{HttpResponse, Responder};

pub async fn support_dashboard() -> impl Responder {
    HttpResponse::Ok().body("Support Dashboard Placeholder")
}

pub async fn submit_reply() -> impl Responder {
    HttpResponse::Ok().body("Reply Submitted Placeholder")
}