//! BibliZap is a library for building citation networks starting from seed articles.
//!
//! It interacts with APIs like Lens.org and PubMed to retrieve article data
//! and expand the network by finding references and citations.
use lens::lensid;

pub mod common;
pub mod lens;
pub mod pubmed;

pub use common::SearchFor;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::lens::{cache::CacheBackend, lensid::LensId};

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    LensError(#[from] lens::error::LensError),
}

/// Represents an article with core bibliographic information.
///
/// This struct is used throughout the library to represent articles
/// retrieved from various sources.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Article {
    pub first_author: Option<String>,
    pub year_published: Option<i32>,
    pub journal: Option<String>,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub doi: Option<String>,
    pub citations: Option<i32>,
    pub score: Option<i32>,
}

impl From<lens::article::Article> for Article {
    fn from(article: lens::article::Article) -> Self {
        Article {
            first_author: article.first_author_name(),
            year_published: article.year_published,
            journal: article.journal(),
            title: article.title.to_owned(),
            summary: article.summary.to_owned(),
            doi: article.doi(),
            citations: article.scholarly_citations_count,
            score: None,
        }
    }
}

impl From<lens::article::ArticleWithData> for Article {
    fn from(article_with_data: lens::article::ArticleWithData) -> Self {
        let article_data = article_with_data.article_data;
        let external_ids = article_data.external_ids;

        let doi = external_ids
            .as_ref()
            .and_then(|ids| ids.doi.first().cloned());

        let first_author = article_data
            .authors
            .as_ref()
            .and_then(|authors| authors.first())
            .map(|author| {
                format!(
                    "{} {}",
                    author.first_name.clone().unwrap_or_default(),
                    author.last_name.clone().unwrap_or_default()
                )
            });

        let journal = article_data
            .source
            .as_ref()
            .and_then(|source| source.title.clone());

        Article {
            first_author,
            year_published: article_data.year_published,
            journal,
            title: article_data.title,
            summary: article_data.summary,
            doi,
            citations: article_data.scholarly_citations_count,
            score: None,
        }
    }
}

/// Expands a citation network starting from a set of seed articles.
///
/// This function performs a "snowballing" process, iteratively finding
/// references and/or citations for the current set of articles and adding
/// new articles to the network until the desired depth is reached or
/// no new articles are found.
///
/// # Arguments
///
/// * `id_list`: A slice of article identifiers (LensIds, PMIDs, DOIs, etc.).
/// * `max_depth`: The maximum depth of the snowballing process. A depth of 0 means only
///   the seed articles are returned. A depth of 1 means seed articles and their
///   direct references/citations are included, and so on.
/// * `output_max_size`: Maximum number of articles to return (top N by score).
/// * `search_for`: Specifies whether to search for references, citations, or both.
/// * `api_key`: The API key for Lens.org. Required for using the Lens.org API.
/// * `client`: Optional `reqwest::Client` for making HTTP requests. If `None`, a new client is created.
///   Pass a custom client to configure proxies, timeouts, headers, etc.
/// * `cache`: Optional cache backend for storing and retrieving data.
///
/// # Returns
///
/// A `Result` containing a `Vec` of `Article` structs sorted by score,
/// or an `Error` if the operation fails.
pub async fn snowball<S>(
    id_list: &[S],
    max_depth: u8,
    output_max_size: usize,
    search_for: &SearchFor,
    api_key: &str,
    client: Option<&reqwest::Client>,
    cache: Option<&dyn CacheBackend>,
) -> Result<Vec<Article>, Error>
where
    S: AsRef<str>,
{
    // Create a client if none provided
    let client_ref = match client {
        Some(c) => c,
        None => &reqwest::Client::new(),
    };

    let snowball_id = lens::snowball(
        id_list,
        max_depth,
        search_for,
        api_key,
        Some(client_ref),
        cache,
    )
    .await?;

    let score_hashmap = snowball_id.into_inner();

    let mut s = score_hashmap.iter().collect::<Vec<_>>();
    s.sort_by_key(|x| std::cmp::Reverse(x.1));
    s.truncate(output_max_size);

    let selected_id: Vec<LensId> = s.iter().map(|(id, _)| (*id).clone()).collect();

    let lens_articles =
        lens::complete_articles(&selected_id, api_key, Some(client_ref), cache).await?;

    let mut articles_kv = lens_articles
        .into_iter()
        .map(|lens_article| (lens_article.lens_id.to_owned(), lens_article.into()))
        .collect::<Vec<(lensid::LensId, Article)>>();

    for (k, v) in articles_kv.iter_mut() {
        v.score = score_hashmap.get(k).map(|x| *x as i32);
    }

    let mut articles = articles_kv
        .into_iter()
        .map(|(_, article)| article)
        .filter(|article| article.score.is_some())
        .collect::<Vec<_>>();

    articles.sort_by_key(|v| v.score.unwrap_or_default());

    Ok(articles)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper function to get API key from environment
    fn get_api_key() -> String {
        dotenvy::var("LENS_API_KEY").expect("LENS_API_KEY must be set in .env file")
    }

    /// Test that the full snowball API (including article completion) works completely offline when cache is populated.
    ///
    /// This validates that:
    /// 1. Cached citation/reference data eliminates the need for network access during snowballing
    /// 2. Cached article data eliminates the need for network access during article completion
    /// 3. The entire public API workflow can operate offline from cache
    #[cfg_attr(feature = "cache-sqlite", tokio::test)]
    async fn test_snowball_offline_with_cache() {
        use crate::lens::cache::sqlite::SqliteBackend;

        let api_key = get_api_key();

        // Create an in-memory cache
        let cache = SqliteBackend::from_url("sqlite::memory:")
            .await
            .expect("Failed to create cache backend");

        // Use a known PMID
        let pmid = "11748933";
        let ids = vec![pmid];

        // Step 1: Populate cache with normal client (online)
        println!("Step 1: Populating cache with snowball() using normal client (online)...");
        let result1 = snowball(
            &ids,
            1,  // depth 1
            20, // get 20 articles
            &SearchFor::References,
            &api_key,
            None, // Use default client
            Some(&cache),
        )
        .await
        .expect("First snowball call with normal client should succeed");

        assert!(
            !result1.is_empty(),
            "Should have fetched and completed articles"
        );
        println!("  ✓ Cached and completed {} articles", result1.len());

        // Verify articles have full data populated
        assert!(
            result1.iter().any(|a| a.title.is_some()),
            "Articles should have titles"
        );
        assert!(
            result1.iter().all(|a| a.score.is_some()),
            "All articles should have scores"
        );

        // Step 2: Create a broken client that cannot make network requests
        println!("Step 2: Creating broken client (simulating offline)...");
        let broken_client = reqwest::Client::builder()
            .proxy(reqwest::Proxy::all("http://0.0.0.0:1").expect("Failed to create invalid proxy"))
            .timeout(std::time::Duration::from_secs(1)) // Fast timeout for quick failure
            .build()
            .expect("Failed to build broken client");

        println!("  ✓ Client configured to fail all network requests");

        // Step 2.5: Verify the broken client actually fails without cache
        println!("Step 2.5: Verifying broken client fails without cache...");
        let verification_result = snowball(
            &[pmid],
            1,
            20,
            &SearchFor::References,
            &api_key,
            Some(&broken_client),
            None, // No cache
        )
        .await;

        assert!(
            verification_result.is_err(),
            "Broken client should fail when data is not in cache"
        );
        println!("  ✓ Confirmed: broken client cannot make network requests");

        // Step 3: Try the same query with broken client + cache - should succeed completely offline!
        println!(
            "Step 3: Attempting same snowball() with broken client + cache (should work offline)..."
        );
        let result2 = snowball(
            &ids,
            1,
            20,
            &SearchFor::References,
            &api_key,
            Some(&broken_client),
            Some(&cache),
        )
        .await
        .expect("Second snowball call should succeed from cache despite broken client");

        assert_eq!(
            result1.len(),
            result2.len(),
            "Both snowball calls should return the same number of articles"
        );

        // Verify the articles have the same scores (proving they came from cache)
        let scores1: Vec<i32> = result1.iter().filter_map(|a| a.score).collect();
        let scores2: Vec<i32> = result2.iter().filter_map(|a| a.score).collect();
        assert_eq!(scores1, scores2, "Scores should be identical from cache");

        // Verify full article data is present (not just IDs)
        assert!(
            result2.iter().any(|a| a.title.is_some()),
            "Cached articles should have complete data including titles"
        );
        assert!(
            result2.iter().any(|a| a.first_author.is_some()),
            "Cached articles should have author information"
        );

        println!("  ✓ Snowball query succeeded using broken client + cache (no network access)");
        println!("  ✓ Article completion worked from cache with broken client");
        println!(
            "\n✓ OFFLINE TEST PASSED: Full public API works offline with broken client when cache is populated!"
        );
    }

    /// Test Article conversion from lens::article::Article
    #[test]
    fn test_article_from_lens_article_conversion() {
        use crate::lens::article::Article as LensArticle;
        use crate::lens::lensid::LensId;

        let lens_article = LensArticle {
            lens_id: LensId::from(12345678901234),
            title: Some("Test Article".to_string()),
            summary: Some("Test summary".to_string()),
            year_published: Some(2020),
            scholarly_citations_count: Some(42),
            external_ids: None,
            authors: None,
            source: None,
        };

        let article: Article = lens_article.into();

        assert_eq!(article.title, Some("Test Article".to_string()));
        assert_eq!(article.summary, Some("Test summary".to_string()));
        assert_eq!(article.year_published, Some(2020));
        assert_eq!(article.citations, Some(42));
        assert_eq!(article.score, None); // Score is not set during conversion
    }
}
