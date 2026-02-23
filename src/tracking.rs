use actix_web::{
    HttpResponse, Responder,
    cookie::{Cookie, SameSite},
    web,
};
use crate::AppConfig;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use uuid::Uuid;

/// Request body for the /link endpoint.
#[derive(Debug, Deserialize)]
pub struct LinkRequest {
    pub biblitest_token: String,
}

/// Response body for the /link endpoint.
#[derive(Debug, Serialize)]
pub struct LinkResponse {
    pub bbz_sid: Uuid,
}

/// Tracking-specific error types.
#[derive(Error, Debug)]
pub enum TrackingError {
    #[error(transparent)]
    DatabaseError(#[from] sqlx::Error),
    #[error("Invalid Biblitest token format: {0}")]
    InvalidBiblitestToken(String),
}

/// Validates a Biblitest token format and CRC32 checksum.
/// Expected format: "BT-{12 alphanumeric}-{2 hex chars}"
/// Example: BT-A1B2C3D4E5F6-7A
pub fn validate_biblitest_token(token: &str) -> Result<(), TrackingError> {
    // Check prefix
    if !token.starts_with("BT-") {
        return Err(TrackingError::InvalidBiblitestToken(
            "Token must start with 'BT-'".to_string(),
        ));
    }

    // Expected length: "BT-" (3) + 12 alphanumeric + "-" (1) + 2 hex = 18 chars
    if token.len() != 18 {
        return Err(TrackingError::InvalidBiblitestToken(format!(
            "Token must be 18 characters, got {}",
            token.len()
        )));
    }

    // Check structure: BT-XXXXXXXXXXXX-YY
    let parts: Vec<&str> = token.split('-').collect();
    if parts.len() != 3 {
        return Err(TrackingError::InvalidBiblitestToken(
            "Token must have format BT-XXXXXXXXXXXX-YY".to_string(),
        ));
    }

    let payload = parts[1];
    let checksum = parts[2];

    // Validate payload (12 alphanumeric characters)
    if payload.len() != 12 || !payload.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(TrackingError::InvalidBiblitestToken(
            "Payload must be 12 alphanumeric characters".to_string(),
        ));
    }

    // Validate checksum (2 hex characters)
    if checksum.len() != 2 || !checksum.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(TrackingError::InvalidBiblitestToken(
            "Checksum must be 2 hexadecimal characters".to_string(),
        ));
    }

    // Compute CRC32 of payload and verify checksum
    let computed_crc = crc32fast::hash(payload.as_bytes());
    let computed_checksum = format!("{:02X}", (computed_crc & 0xFF) as u8);

    if checksum.to_uppercase() != computed_checksum {
        return Err(TrackingError::InvalidBiblitestToken(format!(
            "Invalid checksum: expected {}, got {}",
            computed_checksum, checksum
        )));
    }

    Ok(())
}

/// Generates a new BibliZap session ID (UUID v4).
pub fn generate_bbz_sid() -> Uuid {
    Uuid::new_v4()
}

/// Actix-web handler for the `/link` endpoint.
/// Links a Biblitest token to a BibliZap session ID.
pub async fn link_handler(
    req_body: web::Json<LinkRequest>,
    config: web::Data<AppConfig>,
) -> impl Responder {
    let token = &req_body.biblitest_token;
    let tracking_pool = &config.tracking_pool;

    // Validate token format and checksum
    if let Err(e) = validate_biblitest_token(token) {
        log::warn!("Invalid Biblitest token: {}", e);
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": e.to_string()
        }));
    }

    // Check if token already exists in database
    let existing = sqlx::query!(
        r#"
        SELECT bbz_sid FROM token_link WHERE biblitest_token = $1
        "#,
        token
    )
    .fetch_optional(tracking_pool)
    .await;

    let bbz_sid = match existing {
        Ok(Some(record)) => {
            log::info!("Existing token mapping found");
            record.bbz_sid
        }
        Ok(None) => {
            // Generate new session ID and insert mapping
            let new_sid = generate_bbz_sid();

            let insert_result = sqlx::query!(
                r#"
                INSERT INTO token_link (biblitest_token, bbz_sid)
                VALUES ($1, $2)
                ON CONFLICT (biblitest_token) DO UPDATE
                SET biblitest_token = EXCLUDED.biblitest_token
                RETURNING bbz_sid
                "#,
                token,
                new_sid
            )
            .fetch_one(tracking_pool)
            .await;

            match insert_result {
                Ok(record) => {
                    log::info!("New token mapping created: {} -> {}", token, record.bbz_sid);
                    record.bbz_sid
                }
                Err(e) => {
                    log::error!("Failed to insert token mapping: {}", e);
                    return HttpResponse::InternalServerError().json(serde_json::json!({
                        "error": "Failed to create session"
                    }));
                }
            }
        }
        Err(e) => {
            log::error!("Database query failed: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Database error"
            }));
        }
    };

    // Set cookie with HttpOnly, Secure, SameSite=Lax, Max-Age=7200 (2 hours)
    let cookie = Cookie::build("bbz_sid", bbz_sid.to_string())
        .path("/")
        .max_age(actix_web::cookie::time::Duration::seconds(7200))
        .same_site(SameSite::Lax)
        .secure(true)
        .http_only(true)
        .finish();

    HttpResponse::Ok()
        .cookie(cookie)
        .json(LinkResponse { bbz_sid })
}

/// Logs a successful search event asynchronously.
pub fn log_search_success(
    bbz_sid: Uuid,
    article_count: usize,
    request_started_ms: i64,
    request_completed_ms: i64,
    request_duration_ms: i32,
    request_inputs: Option<Value>,
    pool: sqlx::PgPool,
) {
    tokio::spawn(async move {
        let metadata = serde_json::json!({
            "request": request_inputs,
            "result_count": article_count,
        });

        let result = sqlx::query!(
            r#"
            INSERT INTO bbz_events (
                bbz_sid,
                event_type,
                endpoint,
                request_started_ms,
                request_completed_ms,
                request_duration_ms,
                metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
            bbz_sid,
            "search_success",
            "/api",
            request_started_ms,
            request_completed_ms,
            request_duration_ms,
            metadata
        )
        .execute(&pool)
        .await;

        if let Err(e) = result {
            log::warn!("Failed to log event: {}", e);
        }
    });
}

/// Logs a search error event asynchronously.
pub fn log_search_error(
    bbz_sid: Uuid,
    error_msg: String,
    request_started_ms: i64,
    request_completed_ms: i64,
    request_duration_ms: i32,
    request_inputs: Option<Value>,
    pool: sqlx::PgPool,
) {
    tokio::spawn(async move {
        let metadata = serde_json::json!({
            "request": request_inputs,
            "error": error_msg,
        });

        let result = sqlx::query!(
            r#"
            INSERT INTO bbz_events (
                bbz_sid,
                event_type,
                endpoint,
                request_started_ms,
                request_completed_ms,
                request_duration_ms,
                metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
            bbz_sid,
            "search_error",
            "/api",
            request_started_ms,
            request_completed_ms,
            request_duration_ms,
            metadata
        )
        .execute(&pool)
        .await;

        if let Err(e) = result {
            log::warn!("Failed to log error event: {}", e);
        }
    });
}
