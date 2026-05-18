//! Handles interactions with the PubMed API for retrieving article data
//! and expanding citation networks.
//!
//! Note: PubMed primarily provides article metadata and links to related articles
//! (like PMC articles or articles citing the current one via Europe PMC),
//! but does not offer a direct citation/reference list API like Lens.org.
//! The snowballing functionality here is more limited, focusing on retrieving
//! article details based on PMIDs and finding citing/referenced articles via E-Utilities.

use anyhow::{Context, Result};
use regex::Regex;

/// Macro to construct regex patterns for extracting specific fields from PubMed's raw format.
///
/// The raw PubMed format uses field identifiers (like "PMID", "TI", "AB", "AU")
/// followed by a hyphen and the field content, ending before the next field identifier.
macro_rules! pattern {
    ($feature_identifer:expr) => {{
        concat!(
            "(?s)", // Enable dotall mode ('.' matches newline)
            $feature_identifer,
            "[[:space:]]*-[[:space:]]*(.*?)[A-Z]+[[:space:]]*-" // Capture content between identifier and next uppercase identifier + hyphen
        )
    }};
}

/// Regex pattern string for extracting the PubMed ID (PMID).
static PMID_PATTERN: &str = pattern!("PMID");
/// Regex pattern string for extracting the article title (TI).
static TITLE_PATTERN: &str = pattern!("TI");
/// Regex pattern string for extracting the abstract (AB).
static ABSTRACT_PATTERN: &str = pattern!("AB");
/// Regex pattern string for extracting author names (AU).
static AUTHOR_PATTERN: &str = pattern!("AU");

/// Base URL for finding articles that cite a given PubMed ID (citedin).
static ASC_URL_BASE: &str = "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/elink.fcgi?dbfrom=pubmed&linkname=pubmed_pubmed_citedin&id=";
/// Base URL for finding articles referenced by a given PubMed ID (refs).
static DESC_URL_BASE: &str = "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/elink.fcgi?dbfrom=pubmed&linkname=pubmed_pubmed_refs&id=";

// Lazy static initialization for compiled regex patterns.
lazy_static::lazy_static! {
    /// Compiled regex for extracting PMID.
    static ref PMID_REGEX: Regex = Regex::new(PMID_PATTERN).expect("PMID_REGEX failed to compile");
    /// Compiled regex for extracting Title.
    static ref TITLE_REGEX: Regex = Regex::new(TITLE_PATTERN).expect("TITLE_REGEX failed to compile");
    /// Compiled regex for extracting Abstract.
    static ref ABSTRACT_REGEX: Regex = Regex::new(ABSTRACT_PATTERN).expect("ABSTRACT_REGEX failed to compile");
    /// Compiled regex for extracting Authors.
    static ref AUTHOR_REGEX: Regex = Regex::new(AUTHOR_PATTERN).expect("AUTHOR_REGEX failed to compile");
    /// Compiled regex for extracting <Id> tags from E-Utilities responses.
    static ref ID_REGEX: Regex = Regex::new("(?s)<Id>(.*?)</Id>").expect("AUTHOR_REGEX failed to compile");
}

/// Represents an article with core bibliographic information retrieved from PubMed.
#[derive(Debug, Clone, PartialEq)]
pub struct Article {
    /// The PubMed ID (PMID) of the article.
    pub pmid: String,
    /// The title of the article.
    pub title: Option<String>,
    /// The abstract or summary of the article.
    pub summary: Option<String>,
    /// The list of authors.
    pub authors: Option<Vec<String>>,
}

impl Article {
    /// Parses a single raw article string from PubMed format into an `Article` struct.
    ///
    /// # Arguments
    ///
    /// * `raw_article`: A string slice containing the raw PubMed format for one article.
    ///
    /// # Returns
    ///
    /// A `Result` containing the parsed `Article`, or an error if parsing fails (e.g., missing PMID).
    pub fn from_raw_article(raw_article: &str) -> Result<Article> {
        /// Cleans up extracted feature strings by removing newlines and trimming/collapsing whitespace.
        fn clean_feature(input: &str) -> String {
            /// Trims leading/trailing whitespace and replaces multiple internal spaces with a single space.
            pub fn trim_whitespace(s: &str) -> String {
                let mut new_str = s.trim().to_owned();
                let mut prev = ' '; // The initial value doesn't really matter
                new_str.retain(|ch| {
                    let result = ch != ' ' || prev != ' ';
                    prev = ch;
                    result
                });
                new_str
            }

            let ret = input.replace("\r\n", "");

            trim_whitespace(&ret)
        }

        /// Extracts a single occurrence of a field using a regex.
        fn extract_single(article: &str, regex: &Regex, group: usize) -> Option<String> {
            let string = regex.captures(article)?.get(group)?.as_str();
            Some(clean_feature(string))
        }

        /// Extracts all occurrences of a repeating field (like authors) using a regex.
        fn extract_all(article: &str, regex: &Regex, group: usize) -> Option<Vec<String>> {
            let vec: Option<Vec<String>> = regex
                .captures_iter(article)
                .map(|x| Some(clean_feature(x.get(group)?.as_str())))
                .collect();
            vec
        }

        Ok(Article {
            pmid: extract_single(raw_article, &PMID_REGEX, 1)
                .with_context(|| format!("No PMID found for this article : \n{raw_article}"))?,
            title: extract_single(raw_article, &TITLE_REGEX, 1),
            summary: extract_single(raw_article, &ABSTRACT_REGEX, 1),
            authors: extract_all(raw_article, &AUTHOR_REGEX, 1),
        })
    }

    /// Parses a string containing multiple raw articles from PubMed format.
    ///
    /// Assumes articles are separated by double newlines (`\r\n\r\n`).
    ///
    /// # Arguments
    ///
    /// * `raw_articles`: A string slice containing the raw PubMed format for multiple articles.
    ///
    /// # Returns
    ///
    /// A `Result` containing an iterator over the parsed `Article` structs.
    pub fn from_raw_articles(raw_articles: &str) -> Result<impl Iterator<Item = Article> + '_> {
        Ok(raw_articles
            .split("\r\n\r\n")
            .map(|x| Article::from_raw_article(x).unwrap())) // Note: Using unwrap here might panic on parsing errors
    }

    /// Requests raw article data from PubMed for a list of PMIDs.
    ///
    /// Uses the E-Utilities API to fetch articles in PubMed format.
    ///
    /// # Arguments
    ///
    /// * `src_pmid`: A slice of string slices representing the PMIDs to fetch.
    ///
    /// # Returns
    ///
    /// A `Result` containing the raw response body as a string, or an error if the request fails.
    async fn request_raw_articles(src_pmid: &[&str]) -> Result<String> {
        let url: String = format!(
            "https://pubmed.ncbi.nlm.nih.gov/?term={}&show_snippets=off&format=pubmed&size=200",
            src_pmid.join(",") // Join PMIDs with commas for the query term
        );

        let body = reqwest::get(url).await?.text().await?;

        // Regex to extract the <pre> block containing the PubMed format data
        let body_to_raw_articles: Regex = Regex::new(r"(?s)<pre.*?(PMID.*)</pre>")?;

        Ok(body_to_raw_articles
            .captures(&body)
            .context("Capture failed")?
            .get(1)
            .context("Get group 1 failed")?
            .as_str()
            .to_owned())
    }

    /// Completes the information for a list of articles using the PubMed API.
    ///
    /// This function takes a list of PMIDs and fetches detailed data for them
    /// from PubMed, returning a vector of `Article` structs.
    ///
    /// # Arguments
    ///
    /// * `src_pmid`: A slice of string slices representing the PMIDs to complete.
    ///
    /// # Returns
    ///
    /// A `Result` containing a vector of `Article` structs, or an error if fetching or parsing fails.
    pub async fn complete_articles(src_pmid: &[&str]) -> Result<Vec<Article>> {
        let raw_articles = Article::request_raw_articles(src_pmid).await?;
        let ret = Article::from_raw_articles(&raw_articles)?.collect();
        Ok(ret)
    }
}

/// Performs a single step of snowballing using PubMed E-Utilities (unsafe chunk size).
///
/// This function finds articles that cite or are referenced by the given PMIDs
/// by querying the E-Utilities API. It's marked unsafe because it doesn't handle
/// chunking the input PMIDs, which can lead to URL length limits.
///
/// # Arguments
///
/// * `src_pmid`: A slice of string slices representing the source PMIDs.
///
/// # Returns
///
/// A `Result` containing a vector of strings representing the PMIDs of related articles,
/// or an error if the request or parsing fails.
async fn snowball_onestep_unsafe(src_pmid: &[&str]) -> Result<Vec<String>> {
    let src_pmid_comma = src_pmid.join(",");
    let asc_url: String = format!("{ASC_URL_BASE}{src_pmid_comma}");
    let desc_url: String = format!("{DESC_URL_BASE}{src_pmid_comma}");

    let body_asc: String = reqwest::get(asc_url).await?.text().await?;
    let body_desc: String = reqwest::get(desc_url).await?.text().await?;

    let body: String = [body_desc, body_asc].join("\n");

    // Extract PMIDs from <Id> tags in the response
    let dest_pmid: Result<Vec<_>> = ID_REGEX
        .captures_iter(&body)
        .map(|x| anyhow::Ok(x.get(1).context("Couldn't get ID")?.as_str().to_owned()))
        .skip(src_pmid.len()) // Skip n first as pubmed returns input as output before giving citations
        .collect();

    dest_pmid
}

/// Performs a single step of snowballing using PubMed E-Utilities, handling chunking.
///
/// This function finds articles that cite or are referenced by the given PMIDs
/// by querying the E-Utilities API. It chunks the input PMIDs to avoid URL length limits.
///
/// # Arguments
///
/// * `src_pmid`: A slice of string slices representing the source PMIDs.
///
/// # Returns
///
/// A `Result` containing a vector of strings representing the PMIDs of related articles,
/// or an error if the request or parsing fails.
pub async fn snowball_onestep(src_pmid: &[&str]) -> Result<Vec<String>> {
    let dest_pmid = futures::future::join_all(
        src_pmid
            .chunks(325) // Chunk size to manage URL length
            .map(snowball_onestep_unsafe),
    )
    .await
    .into_iter()
    .collect::<Result<Vec<_>>>()?
    .into_iter()
    .flatten()
    .collect::<Vec<String>>();

    Ok(dest_pmid)
}

/// Performs a snowballing expansion of a citation network starting from PubMed IDs.
///
/// This function iteratively finds references and/or citations for the current set
/// of articles using the PubMed E-Utilities API up to a specified maximum depth.
/// Note that PubMed's E-Utilities primarily provide citing articles and referenced
/// articles via links, not comprehensive lists like some other databases.
///
/// # Arguments
///
/// * `src_pmid`: A slice of string slices representing the starting PMIDs.
/// * `max_depth`: The maximum depth of the snowballing process. Depth 0 returns only the initial IDs.
///
/// # Returns
///
/// A `Result` containing a vector of strings representing the unique PMIDs found
/// during the snowballing, or an error if the process fails.
pub async fn snowball(src_pmid: &[&str], max_depth: u8) -> Result<Vec<String>> {
    let mut all_pmid: Vec<String> = Vec::new();

    let mut current_pmid = src_pmid
        .iter()
        .cloned()
        .map(|x| x.to_owned())
        .collect::<Vec<String>>();

    all_pmid.append(&mut current_pmid.clone());

    for _ in 0..max_depth {
        let current_pmid_refs = current_pmid
            .iter()
            .map(|x| x.as_str())
            .collect::<Vec<&str>>();

        current_pmid = snowball_onestep(&current_pmid_refs).await?;

        all_pmid.append(&mut current_pmid.clone());
    }

    Ok(all_pmid)
}
