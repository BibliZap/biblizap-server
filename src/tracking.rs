use serde_json::Value;

/// Logs a successful search event asynchronously.
pub fn log_search_success(
    article_count: usize,
    request_started_ms: i64,
    request_completed_ms: i64,
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
                event_type,
                endpoint,
                request_started_ms,
                request_completed_ms,
                metadata
            )
            VALUES ($1, $2, $3, $4, $5)
            "#,
            "search_success",
            "/api",
            request_started_ms,
            request_completed_ms,
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
    error_msg: String,
    request_started_ms: i64,
    request_completed_ms: i64,
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
                event_type,
                endpoint,
                request_started_ms,
                request_completed_ms,
                metadata
            )
            VALUES ($1, $2, $3, $4, $5)
            "#,
            "search_error",
            "/api",
            request_started_ms,
            request_completed_ms,
            metadata
        )
        .execute(&pool)
        .await;

        if let Err(e) = result {
            log::warn!("Failed to log error event: {}", e);
        }
    });
}
