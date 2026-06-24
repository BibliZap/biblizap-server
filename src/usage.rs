use std::collections::HashMap;

use crate::AppConfig;
use actix_web::{HttpResponse, Responder, web};

pub async fn db_get_daily_usage(pool: &sqlx::PgPool) -> Result<HashMap<i32, i64>, sqlx::Error> {
    let rows = sqlx::query!(
        r#"
        SELECT 
            to_timestamp(request_started_ms / 1000)::date AS "date_bucket!",
            COUNT(*) as "total_requests!"
        FROM bbz_events
        WHERE event_type = 'search_success'
        GROUP BY 1
        ORDER BY 1 DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    let usage_map: HashMap<i32, i64> = rows
        .into_iter()
        .map(|row| (row.date_bucket.to_julian_day(), row.total_requests))
        .collect();

    Ok(usage_map)
}

pub async fn usage_info_request(config: web::Data<AppConfig>) -> impl Responder {
    let usage_info: Result<HashMap<i32, i64>, sqlx::Error> =
        db_get_daily_usage(&config.database_pool).await;

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
