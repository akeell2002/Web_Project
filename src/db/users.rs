use sqlx::PgPool;
use crate::models::{User, RegisterForm};
use crate::utils::{hash_password, verify_password};

// Create a new user in database
pub async fn create_user(pool: &PgPool, form: &RegisterForm, role: &str) -> Result<User, String> {
    // Check if passwords match
    if form.password != form.confirm_password {
        return Err("Passwords do not match".to_string());
    }
    
    let hashed = hash_password(&form.password)?;
    
    let user = sqlx::query_as!(
        User,
        r#"
        INSERT INTO users (username, email, password_hash, role)
        VALUES ($1, $2, $3, $4)
        RETURNING id, username, email, password_hash, role, created_at
        "#,
        form.username,
        form.email,
        hashed,
        role
    )
    .fetch_one(pool)
    .await
    .map_err(|e| format!("Database error: {}", e))?;
    
    Ok(user)
}

// Find user by username for login
pub async fn find_user_by_username(pool: &PgPool, username: &str) -> Result<Option<User>, String> {
    let user = sqlx::query_as!(
        User,
        r#"
        SELECT id, username, email, password_hash, role, created_at
        FROM users
        WHERE username = $1
        "#,
        username
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Database error: {}", e))?;
    
    Ok(user)
}

// Verify login credentials
pub async fn authenticate_user(pool: &PgPool, username: &str, password: &str) -> Result<Option<User>, String> {
    let user = match find_user_by_username(pool, username).await? {
        Some(u) => u,
        None => return Ok(None),
    };
    
    if verify_password(password, &user.password_hash)? {
        Ok(Some(user))
    } else {
        Ok(None)
    }
}