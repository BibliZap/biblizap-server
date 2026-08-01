use std::time::Duration;

use actix_web::{HttpResponse, Responder, web};
use serde::Serialize;
use sqlx::PgPool;

use crate::AppConfig;

const HEALTH_CHECK_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

async fn check_database(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT 1").execute(pool).await?;
    Ok(())
}

/// Report whether the application and both of its databases are available.
pub async fn health(config: web::Data<AppConfig>) -> impl Responder {
    let checks = async {
        let (tracking, cache) = tokio::join!(
            check_database(&config.database_pool),
            check_database(&config.cache_database_pool),
        );

        tracking.map_err(|error| ("tracking", error))?;
        cache.map_err(|error| ("cache", error))?;
        Ok::<(), (&str, sqlx::Error)>(())
    };

    match tokio::time::timeout(HEALTH_CHECK_TIMEOUT, checks).await {
        Ok(Ok(())) => HttpResponse::Ok()
            .insert_header(("Cache-Control", "no-store"))
            .json(HealthResponse { status: "ok" }),
        Ok(Err((database, error))) => {
            log::warn!("Health check failed for {database} database: {error}");
            HttpResponse::ServiceUnavailable()
                .insert_header(("Cache-Control", "no-store"))
                .json(HealthResponse {
                    status: "unavailable",
                })
        }
        Err(_) => {
            log::warn!(
                "Health check timed out after {} seconds",
                HEALTH_CHECK_TIMEOUT.as_secs()
            );
            HttpResponse::ServiceUnavailable()
                .insert_header(("Cache-Control", "no-store"))
                .json(HealthResponse {
                    status: "unavailable",
                })
        }
    }
}
