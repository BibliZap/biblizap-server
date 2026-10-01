use crate::common::*;
use crate::corpus;
use crate::tracking;

use super::{AppConfig, Error};
use actix_web::{HttpResponse, Responder, web};
use biblizap_rs::{SearchFor, lens::cache::postgres::PostgresBackend};
use serde::Deserialize;

pub const MAX_IDS: usize = 100;

/// Parameters received from the frontend for the snowball search.
#[derive(Debug, Deserialize)]
struct SnowballParameters {
    output_max_size: String,
    depth: u8,
    input_id_list: Vec<String>,
    search_for: SearchFor,
    #[serde(default)]
    exclude_corpus_hashes: Vec<String>,
}

/// Handles the core logic of performing the snowball search using biblizap-rs.
/// Takes the request body (JSON string) and the Lens API key.
/// Returns a JSON string representing the search results or an error.
async fn handle_request(
    req_body: &str,
    lens_api_key: &str,
    cache_backend: &PostgresBackend,
    database_pool: &sqlx::PgPool,
) -> Result<String, Error> {
    let parameters = serde_json::from_str::<SnowballParameters>(req_body)?;
    log::info!("Received request: {:?}", parameters);

    // Server-side validation: enforce the seed limit independently of the frontend.
    if parameters.input_id_list.len() > MAX_IDS {
        return Err(Error::TooManyIds(parameters.input_id_list.len()));
    }

    // Server-side validation: ensure at least one ID
    if parameters.input_id_list.is_empty() {
        return Err(Error::NoValidIds);
    }

    // Server-side validation: check each ID is valid DOI or PMID
    for id in &parameters.input_id_list {
        if !is_valid_id(id) {
            return Err(Error::InvalidIdFormat(id.clone()));
        }
    }
    let excluded_dois =
        corpus::load_excluded_dois(database_pool, &parameters.exclude_corpus_hashes).await?;
    let snowball = biblizap_rs::snowball_with_exclusions(
        &parameters.input_id_list,
        parameters.depth.clamp(1, 2),
        parameters
            .output_max_size
            .parse::<usize>()
            .unwrap_or(usize::MAX)
            .clamp(1, usize::MAX),
        &parameters.search_for,
        lens_api_key,
        None,
        Some(cache_backend),
        &excluded_dois,
    )
    .await?;

    let json_str = serde_json::to_string(&snowball)?;
    log::debug!(
        "Sending {} articles, {} characters response",
        snowball.len(),
        json_str.len()
    );

    Ok(json_str)
}

/// Actix-web handler for the `/api` endpoint.
/// Receives the request body, extracts parameters, performs the snowball search,
/// and returns the results as JSON or an error response.
pub async fn snowball_request(req_body: String, config: web::Data<AppConfig>) -> impl Responder {
    let request_started_ms = epoch_ms();
    let request_inputs = serde_json::from_str::<serde_json::Value>(&req_body).ok();
    let snowball: Result<String, Error> = handle_request(
        &req_body,
        &config.lens_api_key,
        &config.cache_backend,
        &config.database_pool,
    )
    .await;
    let request_completed_ms = epoch_ms();

    let pool = config.database_pool.clone();

    match snowball {
        Ok(snowball) => {
            log::info!("Request completed successfully");

            let article_count = snowball.matches("\"doi\":").count();
            tracking::log_search_success(
                article_count,
                request_started_ms,
                request_completed_ms,
                request_inputs.clone(),
                pool,
            );

            HttpResponse::Ok().body(snowball)
        }
        Err(error) => {
            log::error!("Request failed: {error:?}");

            let error_msg = error.to_string();
            tracking::log_search_error(
                error_msg,
                request_started_ms,
                request_completed_ms,
                request_inputs.clone(),
                pool,
            );

            // Return 400 Bad Request for validation errors, 500 for others
            match error {
                Error::InvalidIdFormat(_)
                | Error::TooManyIds(_)
                | Error::NoValidIds
                | Error::CorpusError(corpus::CorpusError::InvalidExclusionHash)
                | Error::CorpusError(corpus::CorpusError::ExclusionCorpusNotFound) => {
                    HttpResponse::BadRequest().body(format!("{error}"))
                }
                _ => HttpResponse::InternalServerError().body(format!("{error}")),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SnowballParameters;

    #[test]
    fn exclusion_hashes_are_optional_in_api_requests() {
        let base = r#"{"output_max_size":"100","depth":2,"input_id_list":["10.1234/example"],"search_for":"Both"}"#;
        let request: SnowballParameters = serde_json::from_str(base).unwrap();
        assert!(request.exclude_corpus_hashes.is_empty());

        let with_hashes = r#"{"output_max_size":"100","depth":2,"input_id_list":["10.1234/example"],"search_for":"Both","exclude_corpus_hashes":["abc","def"]}"#;
        let request: SnowballParameters = serde_json::from_str(with_hashes).unwrap();
        assert_eq!(request.exclude_corpus_hashes, ["abc", "def"]);
    }
}
