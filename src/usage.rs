use std::collections::HashMap;

use actix_web::{HttpResponse, Responder, web};

use crate::AppConfig;

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::Error),
}

#[derive(Debug, serde::Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
enum SamplingTime {
    Day,
    Week,
    Month,
    Year,
    All,
}

impl SamplingTime {
    fn to_duration(&self) -> std::time::Duration {
        match self {
            SamplingTime::Day => std::time::Duration::from_secs(60 * 60 * 24),
            SamplingTime::Week => std::time::Duration::from_secs(60 * 60 * 24 * 7),
            SamplingTime::Month => std::time::Duration::from_secs(60 * 60 * 24 * 30),
            SamplingTime::Year => std::time::Duration::from_secs(60 * 60 * 24 * 365),
            SamplingTime::All => std::time::Duration::from_secs(u64::MAX), // Special case for all time
        }
    }
}

#[derive(Debug, serde::Deserialize, Clone, PartialEq, Eq, Hash)]
struct UsageInfoParameters {
    sampling_time: SamplingTime,
}

async fn db_get_usage_info(
    pool: &sqlx::PgPool,
    sampling_time: SamplingTime,
) -> Result<HashMap<i64, i64>, sqlx::Error> {
    let rows = sqlx::query!(
        r#"
        SELECT 
            FLOOR(((EXTRACT(EPOCH FROM NOW()) * 1000)::BIGINT - request_started_ms) / $1)::BIGINT AS "time_bucket!",
            COUNT(*) as "total_requests!"
        FROM bbz_events
        WHERE event_type = 'search_success'
        GROUP BY 1
        "#,
        sampling_time.to_duration().as_millis() as i64
    )
    .fetch_all(pool)
    .await?;

    let usage_map: HashMap<i64, i64> = rows
        .into_iter()
        .map(|row| (row.time_bucket, row.total_requests))
        .collect();

    Ok(usage_map)
}

async fn handle_request(
    req_body: &str,
    config: web::Data<AppConfig>,
) -> Result<HashMap<i64, i64>, Error> {
    let parameters = serde_json::from_str::<UsageInfoParameters>(req_body)?;
    let usage_info = db_get_usage_info(&config.database_pool, parameters.sampling_time).await?;
    Ok(usage_info)
}

pub async fn usage_info_request(req_body: String, config: web::Data<AppConfig>) -> impl Responder {
    let usage_info: Result<HashMap<i64, i64>, Error> = handle_request(&req_body, config).await;

    match usage_info {
        Ok(usage_info) => {
            log::info!("Usage info request completed successfully");
            HttpResponse::Ok().json(usage_info)
        }
        Err(error) => {
            log::error!("Usage info request failed: {error:?}");
            HttpResponse::InternalServerError().body(format!("Error: {}", error))
        }
    }
}
