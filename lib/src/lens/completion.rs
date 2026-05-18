use crate::lens::{
    article::{ArticleData, ArticleWithData},
    cache::CacheBackend,
    error::LensError,
    id_types::TypedIdList,
    lensid::LensId,
    request::request_and_parse,
};

/// Completes the information for a list of articles using the Lens.org API.
///
/// This function takes a list of LensIds and fetches detailed article data
/// for them from Lens.org, returning a vector of `ArticleWithData` structs.
/// It handles chunking the requests to avoid hitting API limits and uses
/// caching when available.
///
/// # Arguments
///
/// * `id_list`: A slice of LensIds to fetch article data for.
/// * `api_key`: The API key for Lens.org.
/// * `client`: An optional `reqwest::Client` to use for requests. If `None`, a new client is created.
/// * `cache`: An optional cache backend to use for caching results.
///
/// # Returns
///
/// A `Result` containing a vector of `ArticleWithData` structs, or a `LensError` if an error occurs.
pub async fn complete_articles(
    id_list: &[LensId],
    api_key: &str,
    client: Option<&reqwest::Client>,
    cache: Option<&dyn CacheBackend>,
) -> Result<Vec<ArticleWithData>, LensError> {
    let Some(cache_backend) = cache else {
        return complete_articles_no_cache(id_list, api_key, client).await;
    };

    // Get cached articles
    let mut cached_articles = cache_backend.get_article_data(id_list).await?;

    // Compute which IDs were not found in cache
    let cached_lens_ids: Vec<LensId> = cached_articles
        .iter()
        .map(|article| article.lens_id.clone())
        .collect();

    let cache_misses: Vec<LensId> = id_list
        .iter()
        .filter(|id| !cached_lens_ids.contains(id))
        .cloned()
        .collect();

    if cache_misses.is_empty() {
        return Ok(cached_articles);
    }

    let mut fetched_articles = complete_articles_no_cache(&cache_misses, api_key, client).await?;

    cache_backend.store_article_data(&fetched_articles).await?;

    cached_articles.append(&mut fetched_articles);

    Ok(cached_articles)
}

async fn complete_articles_no_cache(
    id_list: &[LensId],
    api_key: &str,
    client: Option<&reqwest::Client>,
) -> Result<Vec<ArticleWithData>, LensError> {
    let client = match client {
        Some(t) => t.to_owned(),
        None => reqwest::Client::new(),
    };

    let output_id = futures::future::join_all(
        id_list
            .chunks(1000) // Chunk requests to manage load
            .map(|x| request_batch(x, api_key, &client)),
    )
    .await
    .into_iter()
    .collect::<Result<Vec<_>, LensError>>()?
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();

    Ok(output_id)
}

/// Completes the information for a chunk of article IDs using the Lens.org API.
///
/// This is an internal helper function used by `complete_articles`. It categorizes
/// the IDs and makes typed requests to the API.
///
/// # Arguments
///
/// * `id_list`: A slice of items that can be referenced as strings for this chunk.
/// * `api_key`: The API key for Lens.org.
/// * `client`: An optional `reqwest::Client` to use for requests. If `None`, a new client is created.
///
/// # Returns
///
/// A `Result` containing a vector of `Article` structs for this chunk, or a `LensError`.
async fn request_batch<T>(
    id_list: &[T],
    api_key: &str,
    client: &reqwest::Client,
) -> Result<Vec<ArticleWithData>, LensError>
where
    T: AsRef<str>,
{
    let iter = id_list.iter().map(|item| item.as_ref());

    let typed_id_list = TypedIdList::from_raw_id_list(iter.clone())?;

    let mut complete_articles = Vec::<ArticleWithData>::with_capacity(iter.len());

    // Fetch articles by each ID type
    complete_articles.append(
        &mut request_batch_one_id_type(&typed_id_list.pmid, "pmid", api_key, client).await?,
    );
    complete_articles.append(
        &mut request_batch_one_id_type(&typed_id_list.lens_id, "lens_id", api_key, client).await?,
    );
    complete_articles
        .append(&mut request_batch_one_id_type(&typed_id_list.doi, "doi", api_key, client).await?);

    Ok(complete_articles)
}

/// Fetches detailed article information from Lens.org for a list of IDs of a specific type.
///
/// This is an internal helper function used by `complete_articles_chunk`.
///
/// # Arguments
///
/// * `id_list`: A slice of string slices representing IDs of a single type (e.g., all PMIDs).
/// * `id_type`: The type of IDs in `id_list` (e.g., "pmid", "lens_id", "doi").
/// * `api_key`: The API key for Lens.org.
/// * `client`: The `reqwest::Client` to use for the request.
///
/// # Returns
///
/// A `Result` containing a vector of `Article` structs, or a `LensError`.
async fn request_batch_one_id_type(
    id_list: &[&str],
    id_type: &str,
    api_key: &str,
    client: &reqwest::Client,
) -> Result<Vec<ArticleWithData>, LensError> {
    // Fields to include in the API response
    let include = [
        "lens_id",
        "title",
        "authors",
        "abstract",
        "external_ids",
        "scholarly_citations_count",
        "source",
        "year_published",
    ];

    // Fetch articles from API
    let articles: Vec<crate::lens::article::Article> =
        request_and_parse(client, api_key, id_list, id_type, &include).await?;

    // Convert Article to ArticleWithData
    articles
        .into_iter()
        .map(|article| {
            Ok(ArticleWithData {
                lens_id: article.lens_id,
                article_data: ArticleData {
                    title: article.title,
                    summary: article.summary,
                    scholarly_citations_count: article.scholarly_citations_count,
                    external_ids: article.external_ids,
                    authors: article.authors,
                    source: article.source,
                    year_published: article.year_published,
                },
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests the `complete_articles` function by fetching details for known IDs.
    #[tokio::test]
    async fn complete_articles_test() {
        let src_id = [LensId::from(2020040130733), LensId::from(5070897679125)];

        let api_key = dotenvy::var("LENS_API_KEY").expect("LENS_API_KEY must be set in .env file");

        let articles = complete_articles(&src_id, &api_key, None, None)
            .await
            .unwrap();

        assert_eq!(articles.len(), src_id.len());

        for article in articles.into_iter() {
            println!("{article:#?}");
        }
    }

    /// Tests that `complete_articles` correctly uses caching
    #[cfg_attr(feature = "cache-sqlite", tokio::test)]
    async fn complete_articles_with_cache_test() {
        use crate::lens::cache::SqliteBackend;

        let src_id = [
            LensId::from(2020040130733),
            LensId::from(5070897679125),
            LensId::from(10_000_000_000_001), // Non-existent ID to test partial misses
        ];

        let api_key = dotenvy::var("LENS_API_KEY").expect("LENS_API_KEY must be set in .env file");

        // Create an in-memory cache
        let cache = SqliteBackend::from_url("sqlite::memory:")
            .await
            .expect("Failed to create cache backend");

        // First call - should fetch from API and populate cache
        let articles_first = complete_articles(&src_id[..2], &api_key, None, Some(&cache))
            .await
            .unwrap();

        assert_eq!(articles_first.len(), 2);
        println!("First call fetched {} articles", articles_first.len());

        // Verify the articles were stored in cache
        let cached = cache.get_article_data(&src_id[..2]).await.unwrap();
        assert_eq!(cached.len(), 2, "Cache should contain 2 articles");

        // Second call - should retrieve from cache (no API call needed)
        let articles_second = complete_articles(&src_id[..2], &api_key, None, Some(&cache))
            .await
            .unwrap();

        assert_eq!(articles_second.len(), 2);
        println!(
            "Second call retrieved {} articles from cache",
            articles_second.len()
        );

        // Verify the data is the same
        assert_eq!(articles_first[0].lens_id, articles_second[0].lens_id);
        assert_eq!(
            articles_first[0].article_data.title,
            articles_second[0].article_data.title
        );

        // Test partial cache hit - one cached, one new
        let mixed_ids = [src_id[0].clone(), LensId::from(5070897679125)];
        let articles_mixed = complete_articles(&mixed_ids, &api_key, None, Some(&cache))
            .await
            .unwrap();

        assert_eq!(articles_mixed.len(), 2);
        println!(
            "Mixed call (1 cached + 1 fetched) returned {} articles",
            articles_mixed.len()
        );

        // Verify cache now has all fetched articles
        let final_cached = cache.get_article_data(&mixed_ids).await.unwrap();
        assert_eq!(final_cached.len(), 2, "Cache should contain both articles");

        println!("Cache test completed successfully!");
    }
}
