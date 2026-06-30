// To export for use in other modules
pub mod auth;
pub mod admin;
pub mod appointments;
pub mod patients;
pub mod billing;
pub mod beds;

use actix_session::Session;
use actix_web::HttpResponse;
use actix_web::http::StatusCode;

// Utility function to retrieve the display name of the logged-in user from the session
pub fn get_display_name(session: &Session) -> String {
    if let Ok(Some(name)) = session.get::<String>("name") {
        if !name.is_empty() {
            return name;
        }
    }
    session.get::<String>("email")
        .unwrap_or_default()
        .unwrap_or_default()
        .split('@')
        .next()
        .unwrap_or("User")
        .to_string()
}

// Utility function to enforce staff-only access based on session role.
// Any denial will return the styled 403 page.
pub(crate) fn staff_only(session: &Session) -> Result<(), HttpResponse> {
    match session.get::<String>("role") {
        Ok(Some(role)) if matches!(role.as_str(), "doctor" | "nurse" | "receptionist" | "admin") => Ok(()),
        _ => Err(forbidden_page()),
    }
}

// Builds a self-contained, styled error page (used for 403 and 404).
pub(crate) fn error_page(status: StatusCode, code: &str, title: &str, message: &str) -> HttpResponse {
    let html = format!(r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>{code} {title} · HealthHub</title>
<style>
  body{{margin:0;min-height:100vh;display:flex;align-items:center;justify-content:center;
       font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Arial,sans-serif;
       background:linear-gradient(135deg,#eaf1ff 0%,#dbe7ff 100%);color:#0b2a6b;}}
  .err-card{{background:#fff;border-radius:20px;padding:48px 40px;max-width:460px;width:90%;
            text-align:center;box-shadow:0 20px 60px rgba(13,42,107,0.15);}}
  .err-logo{{height:44px;margin-bottom:18px;}}
  .err-code{{font-size:88px;font-weight:800;line-height:1;color:#1b46e3;margin:0;letter-spacing:-2px;}}
  .err-title{{font-size:24px;font-weight:700;margin:10px 0 8px;}}
  .err-msg{{font-size:15px;color:#5a6b8c;margin:0 0 28px;line-height:1.5;}}
  .err-actions{{display:flex;gap:12px;justify-content:center;flex-wrap:wrap;}}
  .err-btn{{display:inline-block;padding:12px 22px;border-radius:10px;font-weight:600;
           font-size:14px;text-decoration:none;cursor:pointer;border:none;}}
  .err-btn-primary{{background:#1b46e3;color:#fff;}}
  .err-btn-primary:hover{{background:#1536b8;}}
  .err-btn-ghost{{background:#eef2fb;color:#1b46e3;}}
  .err-btn-ghost:hover{{background:#e0e7f8;}}
</style>
</head>
<body>
  <div class="err-card">
    <img src="/static/logo.png" alt="HealthHub" class="err-logo" onerror="this.style.display='none'">
    <p class="err-code">{code}</p>
    <h1 class="err-title">{title}</h1>
    <p class="err-msg">{message}</p>
    <div class="err-actions">
      <a href="#" class="err-btn err-btn-primary" onclick="history.back(); return false;">&larr; Back to previous page</a>
      <a href="/" class="err-btn err-btn-ghost">Go to Homepage</a>
    </div>
  </div>
</body>
</html>"##);
    HttpResponse::build(status)
        .content_type("text/html; charset=utf-8")
        .body(html)
}

// 403 Forbidden page for access-control denials.
pub(crate) fn forbidden_page() -> HttpResponse {
    error_page(
        StatusCode::FORBIDDEN,
        "403",
        "Access Forbidden",
        "You don't have permission to view this page. Please log in with an authorised account.",
    )
}

// 404 Not Found page.
pub(crate) fn not_found_page() -> HttpResponse {
    error_page(
        StatusCode::NOT_FOUND,
        "404",
        "Page Not Found",
        "The page you're looking for doesn't exist or may have moved.",
    )
}

// Catch-all handler wired as the app's default service for unmatched routes.
pub(crate) async fn not_found_handler() -> HttpResponse {
    not_found_page()
}

