use crate::lens::error::LensApiErrorInfo;

use super::error::*;
use serde::Deserialize;

pub const INCLUDES_SEARCH_FOR_CITATIONS_AND_REFERENCES: [&str; 3] =
    ["lens_id", "references", "scholarly_citations"];

pub const INCLUDES_SEARCH_FOR_CITATIONS: [&str; 2] = ["lens_id", "scholarly_citations"];

/// Makes a POST request to the Lens.org API's scholarly search endpoint.
///
/// This function constructs the request body based on the provided IDs, ID type,
/// and fields to include, and then sends the request using `request_response_with_body`.
///
/// # Arguments
///
/// * `client`: The `reqwest::Client` to use for the request.
/// * `api_key`: The API key for Lens.org.
/// * `id_list`: An iterator of IDs to search for. Must be serializable.
/// * `id_type`: The type of IDs in `id_list` (e.g., "pmid", "lens_id", "doi").
/// * `include`: A slice of strings specifying which fields to include in the response.
///
/// # Returns
///
/// A `Result` containing the `reqwest::Response` if successful, or a `LensError` if an error occurs.
pub async fn request_response(
    client: &reqwest::Client,
    api_key: &str,
    id_list: impl IntoIterator<Item = impl serde::Serialize> + serde::Serialize,
    id_type: &str,
    include: &[&str],
) -> Result<reqwest::Response, LensError> {
    request_response_with_body(
        client,
        api_key,
        &make_request_body(id_list, id_type, include),
    )
    .await
}

/// Sends a POST request with a pre-built JSON body to the Lens.org API.
///
/// This function handles sending the HTTP request, adding necessary headers
/// (Authorization, Content-Type), and retrying the request if a rate limit
/// error is encountered based on the `x-rate-limit-retry-after-seconds` header.
///
/// # Arguments
///
/// * `client`: The `reqwest::Client` to use for the request.
/// * `api_key`: The API key for Lens.org.
/// * `body`: The JSON request body as a string.
///
/// # Returns
///
/// A `Result` containing the `reqwest::Response` if successful (status 200),
/// or a `LensError` if an error occurs (e.g., HTTP error, missing rate limit header, parse error).
async fn request_response_with_body(
    client: &reqwest::Client,
    api_key: &str,
    body: &str,
) -> Result<reqwest::Response, LensError> {
    let base_url: &str = "https://api.lens.org/scholarly/search";

    loop {
        let response = client
            .post(base_url)
            .header("Authorization", api_key)
            .header("Content-Type", "application/json")
            .body(body.to_owned())
            .send()
            .await?;

        if response.status() == 200 {
            return Ok(response);
        } else {
            let header_value = response
                .headers()
                .get("x-rate-limit-retry-after-seconds")
                .ok_or(LensError::LensApi(LensApiErrorInfo {
                    status_code: response.status().as_u16(),
                    message: format!("{:#?}", response.headers()),
                }))?;

            let seconds_to_wait = seconds_to_wait_from_response(header_value)?;

            log::debug!("Told to wait for {seconds_to_wait} seconds");

            async_std::task::sleep(std::time::Duration::from_secs(seconds_to_wait)).await;
        }
    }
}

/// Extracts the number of seconds to wait from the `x-rate-limit-retry-after-seconds` header.
///
/// This function converts the header value to a string and then parses it as a `u64`.
///# Arguments
/// * `header_value`: The header value to extract the wait time from.
///# Returns
/// A `Result` containing the number of seconds to wait if successful,
/// or a `RateLimitExtractionError` if an error occurs (e.g., conversion or parsing error).
fn seconds_to_wait_from_response(
    header_value: &reqwest::header::HeaderValue,
) -> Result<u64, RateLimitExtractionError> {
    Ok(header_value.to_str()?.parse::<u64>()?)
}

/// Wrapper struct for Lens.org API responses.
///
/// The Lens API returns data in a wrapper object with a "data" field containing
/// the actual array of results. This struct allows for efficient direct deserialization.
#[derive(Debug, Deserialize)]
struct LensApiResponse<T> {
    data: Vec<T>,
}

/// Requests data from Lens.org API and parses it with optimized direct deserialization.
///
/// This is a higher-level wrapper around `request_response` that automatically handles
/// JSON parsing using direct deserialization (no intermediate `serde_json::Value`, no clones).
///
/// # Performance
///
/// This function is optimized for performance:
/// - Parses JSON directly to the target type (no intermediate `Value`)
/// - Avoids cloning the response data
/// - Single parse pass instead of two
///
/// # Type Safety Warning
///
/// ⚠️ **IMPORTANT**: The type `T` must match the fields specified in `include`.
///
/// For example:
/// - If `include` contains `["lens_id", "scholarly_citations"]`, then `T` should have
///   a `scholarly_citations` field (or mark it with `#[serde(default)]` to allow missing).
/// - If `include` contains `["lens_id", "title", "authors"]`, then `T` should have
///   `title` and `authors` fields.
///
/// Mismatches between `T` and `include` will cause deserialization errors at runtime.
/// Use `#[serde(default)]` on optional fields to handle missing data gracefully.
///
/// # Arguments
///
/// * `client`: The `reqwest::Client` to use for the request.
/// * `api_key`: The API key for Lens.org.
/// * `id_list`: An iterator of IDs to search for. Must be serializable.
/// * `id_type`: The type of IDs in `id_list` (e.g., "pmid", "lens_id", "doi").
/// * `include`: A slice of strings specifying which fields to include in the response.
///
/// # Returns
///
/// A `Result` containing a `Vec<T>` of parsed results, or a `LensError`.
///
/// # Examples
///
/// ```ignore
/// // Request citations
/// let citations: Vec<ArticleWithCitations> = request_and_parse(
///     &client,
///     &api_key,
///     &["some-lens-id"],
///     "lens_id",
///     &["lens_id", "scholarly_citations"]
/// ).await?;
///
/// // Request complete articles
/// let articles: Vec<Article> = request_and_parse(
///     &client,
///     &api_key,
///     &["some-lens-id"],
///     "lens_id",
///     &["lens_id", "title", "authors", "abstract"]
/// ).await?;
/// ```
pub async fn request_and_parse<T>(
    client: &reqwest::Client,
    api_key: &str,
    id_list: impl IntoIterator<Item = impl serde::Serialize> + serde::Serialize,
    id_type: &str,
    include: &[&str],
) -> Result<Vec<T>, LensError>
where
    T: serde::de::DeserializeOwned,
{
    let response = request_response(client, api_key, id_list, id_type, include).await?;
    let json_str = response.text().await?;

    // Direct deserialization - no intermediate Value, no clone!
    let parsed: LensApiResponse<T> = serde_json::from_str(&json_str)?;
    Ok(parsed.data)
}

/// Constructs the JSON request body for a Lens.org scholarly search API request.
///
/// The body includes the search query based on the provided IDs and type,
/// the list of fields to include in the response, and the requested size
/// (number of results).
///
/// # Arguments
///
/// * `id_list`: An iterator of IDs to include in the query. Must be serializable.
/// * `id_type`: The type of IDs in `id_list` (e.g., "pmid", "lens_id", "doi").
/// * `include`: A slice of strings specifying which fields to include in the response.
///
/// # Returns
///
/// A `String` containing the JSON request body.
fn make_request_body(
    id_list: impl IntoIterator<Item = impl serde::Serialize> + serde::Serialize,
    id_type: &str,
    include: &[&str],
) -> String {
    serde_json::json!(
    {
        "query": {
        "terms": {
        id_type: id_list
    }
    },
        "include": include,
        "size": id_list.into_iter().count()
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test requesting scholarly_citations for PMID 2231712.
    /// This PMID has ~120k citations, so we need to see how Lens responds
    /// and whether we need to implement filtering.
    #[tokio::test]
    #[ignore]
    async fn test_high_citation_pmid() -> Result<(), LensError> {
        let api_key = match dotenvy::var("LENS_API_KEY") {
            Ok(key) => key,
            Err(_) => {
                println!("Skipping test: LENS_API_KEY not set");
                return Ok(());
            }
        };

        let client = reqwest::Client::new();
        let pmid = "2231712";

        println!("\n=== Testing PMID {} (high citation count) ===", pmid);

        // Request with lens_id and scholarly_citations
        let include = vec!["lens_id", "scholarly_citations"];

        let json_str = request_response(&client, &api_key, &[pmid], "pmid", &include)
            .await?
            .text()
            .await?;

        let parsed = serde_json::json!(&json_str);

        println!("{:#?}", parsed);

        Ok(())
    }
}
