use super::AppConfig;
use actix_web::{HttpResponse, Responder, web};
use serde::{Deserialize, Serialize};

/// Parameters received for a PubMed keyword search.
#[derive(Debug, Deserialize)]
struct PubmedSearchParams {
    query: String,
    max_results: Option<usize>,
}

/// A single article result from a PubMed keyword search.
#[derive(Debug, Serialize, Deserialize, Clone)]
struct PubmedSearchResult {
    pmid: String,
    title: Option<String>,
    authors: Option<String>,
    journal: Option<String>,
    year: Option<String>,
    doi: Option<String>,
}

/// Actix-web handler for the `/api/pubmed_search` endpoint.
/// Receives a keyword query, searches PubMed via ESearch+ESummary,
/// and returns matching articles for the user to select from.
pub async fn pubmed_search(req_body: String, config: web::Data<AppConfig>) -> impl Responder {
    let params: PubmedSearchParams = match serde_json::from_str(&req_body) {
        Ok(p) => p,
        Err(e) => return HttpResponse::BadRequest().body(format!("Invalid request: {e}")),
    };

    let max_results = params.max_results.unwrap_or(20).clamp(1, 100);
    let query_encoded = urlencoding::encode(&params.query);

    // Build ESearch URL
    let mut esearch_url = format!(
        "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esearch.fcgi?db=pubmed&retmode=json&term={}&retmax={}&sort=relevance",
        query_encoded, max_results
    );
    if let Some(ref api_key) = config.pubmed_api_key {
        esearch_url.push_str(&format!("&api_key={}", api_key));
    }

    log::info!("PubMed ESearch for: {}", params.query);

    // Call ESearch to get PMIDs
    let esearch_response = match reqwest::get(&esearch_url).await {
        Ok(r) => r,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .body(format!("ESearch request failed: {e}"));
        }
    };
    let esearch_text = match esearch_response.text().await {
        Ok(t) => t,
        Err(e) => {
            return HttpResponse::InternalServerError().body(format!("ESearch read failed: {e}"));
        }
    };
    let esearch_json: serde_json::Value = match serde_json::from_str(&esearch_text) {
        Ok(v) => v,
        Err(e) => {
            return HttpResponse::InternalServerError().body(format!("ESearch parse failed: {e}"));
        }
    };

    let pmids: Vec<String> = match esearch_json["esearchresult"]["idlist"].as_array() {
        Some(arr) => arr
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
        None => return HttpResponse::Ok().json(Vec::<PubmedSearchResult>::new()),
    };

    if pmids.is_empty() {
        return HttpResponse::Ok().json(Vec::<PubmedSearchResult>::new());
    }

    // Build ESummary URL
    let pmid_list = pmids.join(",");
    let mut esummary_url = format!(
        "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esummary.fcgi?db=pubmed&retmode=json&id={}",
        pmid_list
    );
    if let Some(ref api_key) = config.pubmed_api_key {
        esummary_url.push_str(&format!("&api_key={}", api_key));
    }

    // Call ESummary to get article metadata
    let esummary_response = match reqwest::get(&esummary_url).await {
        Ok(r) => r,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .body(format!("ESummary request failed: {e}"));
        }
    };
    let esummary_text = match esummary_response.text().await {
        Ok(t) => t,
        Err(e) => {
            return HttpResponse::InternalServerError().body(format!("ESummary read failed: {e}"));
        }
    };
    let esummary_json: serde_json::Value = match serde_json::from_str(&esummary_text) {
        Ok(v) => v,
        Err(e) => {
            return HttpResponse::InternalServerError().body(format!("ESummary parse failed: {e}"));
        }
    };

    // Parse results preserving ESearch order
    let result_obj = &esummary_json["result"];
    let mut results: Vec<PubmedSearchResult> = Vec::new();
    for pmid in &pmids {
        if let Some(article) = result_obj.get(pmid) {
            let authors = article["authors"].as_array().map(|authors| {
                authors
                    .iter()
                    .filter_map(|a| a["name"].as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            });

            let doi = article["articleids"].as_array().and_then(|ids| {
                ids.iter()
                    .find(|id| id["idtype"].as_str() == Some("doi"))
                    .and_then(|id| id["value"].as_str().map(String::from))
            });

            results.push(PubmedSearchResult {
                pmid: pmid.clone(),
                title: article["title"].as_str().map(String::from),
                authors,
                journal: article["fulljournalname"].as_str().map(String::from),
                year: article["pubdate"]
                    .as_str()
                    .map(|d| d.chars().take(4).collect()),
                doi,
            });
        }
    }

    log::info!("PubMed search returned {} results", results.len());
    HttpResponse::Ok().json(results)
}
