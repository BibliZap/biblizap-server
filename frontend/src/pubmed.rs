use crate::common::Error;
use std::ops::Deref;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::search::denylist::upload_denylist_to_backend;

/// Fetches up to 500 PMIDs for a PubMed keyword search via NCBI ESearch.
/// PMIDs are sent directly to the backend, which resolves them via Lens.org.
pub async fn get_pubmed_pmids(query: &str) -> Result<Vec<String>, Error> {
    let query_encoded = js_sys::encode_uri_component(query);
    let url = format!(
        "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esearch.fcgi?db=pubmed&retmode=json&sort=relevance&retmax=500&term={}",
        query_encoded
    );
    let text = gloo_net::http::Request::get(&url)
        .send()
        .await?
        .text()
        .await?;
    let json: serde_json::Value = serde_json::from_str(&text)?;
    let pmids = json["esearchresult"]["idlist"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    Ok(pmids)
}

enum PageState {
    Loading,
    Error(String),
}

/// The PubMed results page.
/// Reads `?q=` from the URL, fetches DOIs from PubMed, uploads them as a corpus,
/// then navigates directly to seed selection.
#[function_component(PubMedResultsPage)]
pub fn pubmed_results_page() -> Html {
    use crate::common::{FormPosition, Route, SeedSelectionQuery};
    use crate::results::{ErrorMessage, Spinner};
    use crate::search::{BiblizapSearchBar, PubMedResultsQuery};

    let location = use_location();
    let navigator = use_navigator().unwrap();

    let query = location
        .as_ref()
        .and_then(|l| l.query::<PubMedResultsQuery>().ok())
        .map(|q| q.q)
        .unwrap_or_default();

    if query.is_empty() {
        navigator.replace(&Route::BibliZapSearch);
    }

    let form_position = location
        .as_ref()
        .and_then(|l| l.state::<FormPosition>())
        .map(|s| s.deref().to_owned())
        .unwrap_or_default();

    let form_class = form_position.get_class();

    let page_state: UseStateHandle<PageState> = use_state(|| PageState::Loading);

    {
        let page_state = page_state.clone();
        let navigator = navigator.clone();
        let query = query.clone();
        use_effect_with(query.clone(), move |_| {
            if !query.is_empty() {
                spawn_local(async move {
                    let dois = match get_pubmed_pmids(&query).await {
                        Ok(d) if d.is_empty() => {
                            page_state.set(PageState::Error(
                                "No articles found for this query.".to_string(),
                            ));
                            return;
                        }
                        Ok(d) => d,
                        Err(e) => {
                            page_state.set(PageState::Error(format!("PubMed search failed: {e}")));
                            return;
                        }
                    };

                    match upload_denylist_to_backend(dois).await {
                        Ok(hash) => {
                            let _ = navigator.push_with_query(
                                &Route::SeedSelection,
                                &SeedSelectionQuery {
                                    bibliography: hex::encode(hash),
                                },
                            );
                        }
                        Err(e) => {
                            page_state.set(PageState::Error(format!("Upload failed: {e}")));
                        }
                    }
                });
            }
            || ()
        });
    }

    let content = match &*page_state {
        PageState::Loading => html! { <Spinner /> },
        PageState::Error(msg) => html! { <ErrorMessage msg={msg.clone()} /> },
    };

    html! {
        <div>
            <div class={form_class}>
                <BiblizapSearchBar position={FormPosition::Top} value={query.clone()} />
            </div>
            <div class="results-fade-in">
                {content}
            </div>
        </div>
    }
}
