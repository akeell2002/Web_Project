use sqlx::PgPool;
use crate::models::user::{User, UserRole};
use crate::utils::{hash_password, verify_password};

/// Create a new user in the database
pub async fn create_user(pool: &PgPool, email: &str, raw_password: &str, role: UserRole) -> Result<User, String> {
    // Hash the plain-text password using utils.rs
    let hashed_password = hash_password(raw_password)?;
    
    // Perform compile-time checked insert matching our new schema
    // Note: SQLx automatically converts custom Postgres Enums if the type implements sqlx::Type
    let user = sqlx::query_as!(
        User,
        r#"
        INSERT INTO users (email, password, role)
        VALUES ($1, $2, $3::user_role)
        RETURNING id, email, password, role as "role: UserRole", created_at as "created_at!", updated_at as "updated_at!"
        "#,
        email,
        hashed_password,
        role as UserRole
    )
    .fetch_one(pool)
    .await
    .map_err(|e| format!("Database error while creating user: {}", e))?;
    
    Ok(user)
}

/// Find a user strictly by their email (since we use email instead of username for logins)
pub async fn find_user_by_email(pool: &PgPool, email: &str) -> Result<Option<User>, String> {
    let user = sqlx::query_as!(
        User,
        r#"
        SELECT id, email, password, role as "role: UserRole", created_at as "created_at!", updated_at as "updated_at!"
        FROM users
        WHERE email = $1
        "#,
        email
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Database error while retrieving user: {}", e))?;
    
    Ok(user)
}

/// Authenticate user via email and plain-text password
pub async fn authenticate_user(pool: &PgPool, email: &str, password: &str) -> Result<Option<User>, String> {
    // 1. Locate the user profile
    let user = match find_user_by_email(pool, email).await? {
        Some(u) => u,
        None => return Ok(None), // User matching this email doesn't exist
    };
    
    // 2. Use our simplified verify_password function from utils.rs
    if verify_password(password, &user.password) {
        Ok(Some(user))
    } else {
        Ok(None) // Password mismatch
    }
}

/// Fetch all users in the system sorted by creation date
pub async fn get_all_users(pool: &PgPool) -> Result<Vec<User>, sqlx::Error> {
    let users = sqlx::query_as!(
        User,
        r#"
        SELECT id, email, password, role as "role: UserRole", created_at as "created_at!", updated_at as "updated_at!"
        FROM users
        ORDER BY created_at ASC
        "#
    )
    .fetch_all(pool)
    .await?;

    Ok(users)
}

/// Specialized helper query to fetch all doctors for appointment scheduling dropdowns
pub async fn get_all_doctors(pool: &PgPool) -> Result<Vec<User>, sqlx::Error> {
    let doctors = sqlx::query_as!(
        User,
        r#"
        SELECT id, email, password, role as "role: UserRole", created_at as "created_at!", updated_at as "updated_at!"
        FROM users
        WHERE role = 'doctor'::user_role
        ORDER BY email ASC
        "#
    )
    .fetch_all(pool)
    .await?;

    Ok(doctors)
}

// Function to initialise admin user if not already present in the database
pub async fn seed_admin_user(pool: &PgPool) -> Result<(), String> {
    // 1. Check if the admin account already exists
    let admin_exists = sqlx::query!(
        r#"SELECT id FROM users WHERE LOWER(email) = 'admin@clinic.com'"#
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Database verification failed: {}", e))?;

    if admin_exists.is_none() {
        println!("Seeding layer: Admin account not found. Creating clean entry...");

        // 2. Hash the password directly using your utils utility crate
        let raw_password = "faipi";
        let database_secure_hash = hash_password(raw_password)?;

        // 3. Generate a clean random UUID for the admin identifier
        let admin_id = uuid::Uuid::new_v4();

        // 4. Securely write the record to your live database schema
        sqlx::query!(
            r#"
            INSERT INTO users (id, email, password, role)
            VALUES ($1, $2, $3, 'admin'::user_role)
            "#,
            admin_id,
            "admin@clinic.com",
            database_secure_hash
        )
        .execute(pool)
        .await
        .map_err(|e| format!("Failed to write seed record: {}", e))?;

        println!("Seeding layer: Admin account successfully deployed!");
    } else {
        println!("Seeding layer: Admin account already exists. Skipping deployment.");
    }

    Ok(())
}