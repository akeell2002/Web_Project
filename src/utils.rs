use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

/// Hash a password before storing it in the database
pub fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    
    argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| format!("Password hashing failed: {}", e))
        .map(|hash| hash.to_string())
}

/// Verify a plain-text password against a stored database hash.
/// Returns true if valid, false if invalid or corrupted.
pub fn verify_password(password: &str, hash: &str) -> bool {
    // If the hash in the DB is corrupted and fails to parse, treat it as an auth failure
    let parsed_hash = match PasswordHash::new(hash) {
        Ok(h) => h,
        Err(_) => return false,
    };

    let argon2 = Argon2::default();
    argon2.verify_password(password.as_bytes(), &parsed_hash).is_ok()
}