use crate::common::Error;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::ops::Deref;
use yew::prelude::*;
use yew_router::prelude::*;

/// A single article result from a PubMed keyword search.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct PubmedSearchResult {
    pub pmid: String,
    pub title: Option<String>,
    pub authors: Option<String>,
    pub journal: Option<String>,
    pub year: Option<String>,
    pub doi: Option<String>,
}

/// Sends a PubMed keyword search request to the backend.
pub async fn get_pubmed_results(query: &str) -> Result<Vec<PubmedSearchResult>, Error> {
    use gloo_utils::document;
    let url = document().document_uri();
    let url = match url {
        Ok(href) => Ok(href),
        Err(err) => Err(Error::JsValueString(err.as_string().unwrap_or_default())),
    }?
    .replace('#', "");

    let mut api_url = url::Url::parse(&url)?;
    api_url.set_fragment("".into());
    api_url.set_query("".into());
    api_url.set_path("api/pubmed_search");

    let body = serde_json::json!({
        "query": query,
        "max_results": 20
    });

    let response = gloo_net::http::Request::post(api_url.as_str())
        .header("Access-Control-Allow-Origin", "*")
        .body(serde_json::to_string(&body)?)?
        .send()
        .await?;

    let result_text = response.text().await?;

    if !response.ok() {
        return Err(Error::Api(result_text));
    }

    let results: Vec<PubmedSearchResult> = serde_json::from_str(&result_text)?;
    Ok(results)
}

/// Properties for the PubMedResults component.
#[derive(Clone, PartialEq, Properties)]
pub struct PubmedResultsProps {
    /// The list of PubMed search results to display.
    pub results: Vec<PubmedSearchResult>,
    /// Callback when the user clicks "Run BibliZap with selected articles".
    /// Passes a list of identifiers (DOI preferred, PMID as fallback).
    pub on_run_snowball: Callback<Vec<String>>,
}

/// Component for displaying PubMed keyword search results with selection checkboxes.
/// Users can select articles and then run BibliZap snowball on the selected ones.
#[function_component(PubMedResultsView)]
pub fn pubmed_results_view(props: &PubmedResultsProps) -> Html {
    let selected_set = use_state(|| HashSet::<String>::new());

    let toggle_all = {
        let selected = selected_set.clone();
        let results = props.results.clone();
        Callback::from(move |_: MouseEvent| {
            let mut current = (*selected).clone();
            if current.len() == results.len() {
                current.clear();
            } else {
                current = results.iter().map(|r| r.pmid.clone()).collect();
            }
            selected.set(current);
        })
    };

    let on_run = {
        let selected = selected_set.clone();
        let results = props.results.clone();
        let on_run_snowball = props.on_run_snowball.clone();
        Callback::from(move |_: MouseEvent| {
            // For each selected PMID, prefer DOI if available (Lens.org has better DOI coverage)
            let ids: Vec<String> = results
                .iter()
                .filter(|r| selected.contains(&r.pmid))
                .map(|r| r.doi.clone().unwrap_or_else(|| r.pmid.clone()))
                .collect();
            on_run_snowball.emit(ids);
        })
    };

    let all_selected = selected_set.len() == props.results.len() && !props.results.is_empty();
    let has_selection = !selected_set.is_empty();

    html! {
        <div class="container-fluid">
            <hr/>
            <div class="d-flex justify-content-between align-items-center mb-3">
                <h5 class="mb-0">
                    {format!("PubMed search returned {} results", props.results.len())}
                </h5>
                <div class="d-flex gap-2 align-items-center">
                    <button
                        type="button"
                        class="btn btn-outline-secondary btn-sm"
                        onclick={toggle_all}
                    >
                        { if all_selected { "Deselect all" } else { "Select all" } }
                    </button>
                    <button
                        type="button"
                        class={classes!(
                            "btn", "btn-primary",
                            if !has_selection { Some("disabled") } else { None }
                        )}
                        onclick={on_run.clone()}
                        disabled={!has_selection}
                    >
                        <i class="bi bi-search"></i>
                        { format!(" Run BibliZap with {} selected", selected_set.len()) }
                    </button>
                </div>
            </div>

            <div class="table-responsive">
                <table class="table table-hover table-bordered">
                    <thead>
                        <tr>
                            <th style="width:3%"></th>
                            <th>{"PMID"}</th>
                            <th style="width:35%">{"Title"}</th>
                            <th>{"Authors"}</th>
                            <th>{"Journal"}</th>
                            <th>{"Year"}</th>
                        </tr>
                    </thead>
                    <tbody class="table-group-divider">
                        { props.results.iter().map(|article| {
                            html! {
                                <Item result={article.clone()} selected_set={selected_set.clone()} />
                            }
                        }).collect::<Html>() }
                    </tbody>
                </table>
            </div>

            if has_selection {
                <div class="d-flex justify-content-center mt-3 mb-3">
                    <button
                        type="button"
                        class="btn btn-primary btn-lg"
                        onclick={on_run.clone()}
                    >
                        <i class="bi bi-search"></i>
                        { format!(" Run BibliZap with {} selected article{}", selected_set.len(), if selected_set.len() > 1 { "s" } else { "" }) }
                    </button>
                </div>
            }
        </div>
    }
}

/// Properties for the PubMedResults component.
#[derive(Clone, PartialEq, Properties)]
pub struct ItemProps {
    pub result: PubmedSearchResult,
    pub selected_set: UseStateHandle<HashSet<String>>,
}

#[function_component(Item)]
fn item(props: &ItemProps) -> Html {
    let result = &props.result;
    let selected_set = props.selected_set.clone();
    let pmid = result.pmid.clone();
    let is_selected = selected_set.contains(&pmid);
    let toggle = {
        let selected = selected_set.clone();
        let pmid = pmid.clone();
        Callback::from(move |_: MouseEvent| {
            let mut current = (*selected).clone();
            if current.contains(&pmid) {
                current.remove(&pmid);
            } else {
                current.insert(pmid.clone());
            }
            selected.set(current);
        })
    };

    let row_class = if is_selected { "table-primary" } else { "" };

    html! {
        <tr class={row_class} style="cursor:pointer" onclick={toggle.clone()}>
            <td class="text-center">
                <input
                    type="checkbox"
                    class="form-check-input"
                    checked={is_selected}
                    onclick={toggle}
                />
            </td>
            <td>
                <a href={format!("https://pubmed.ncbi.nlm.nih.gov/{}/", result.pmid)}
                   target="_blank"
                   onclick={Callback::from(|e: MouseEvent| e.stop_propagation())}
                >
                    {&result.pmid}
                </a>
            </td>
            <td>{result.title.as_deref().unwrap_or("—")}</td>
            <td>
                <small>{result.authors.as_deref().unwrap_or("—")}</small>
            </td>
            <td>
                <small><em>{result.journal.as_deref().unwrap_or("—")}</em></small>
            </td>
            <td>{result.year.as_deref().unwrap_or("—")}</td>
        </tr>
    }
}

enum PubmedFetchStatus {
    Loading,
    Success(Vec<PubmedSearchResult>),
    Error(Error),
}

/// The PubMed results page.
/// Reads `?q=` from the URL, fetches PubMed search results on mount, and lets the user
/// select articles to launch a BibliZap snowball search.
#[function_component(PubMedResultsPage)]
pub fn pubmed_results_page() -> Html {
    use crate::common::{BibliZapResultsQuery, FormPosition, Route};
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

    let fetch_status: UseStateHandle<PubmedFetchStatus> = use_state(|| PubmedFetchStatus::Loading);

    {
        let fetch_status = fetch_status.clone();
        let query = query.clone();
        use_effect_with(query.clone(), move |_| {
            if !query.is_empty() {
                fetch_status.set(PubmedFetchStatus::Loading);
                wasm_bindgen_futures::spawn_local(async move {
                    let result = get_pubmed_results(&query).await;
                    match result {
                        Ok(results) => fetch_status.set(PubmedFetchStatus::Success(results)),
                        Err(e) => fetch_status.set(PubmedFetchStatus::Error(e)),
                    }
                });
            }
            || ()
        });
    }

    let on_run_snowball = {
        let navigator = navigator.clone();
        Callback::from(move |ids: Vec<String>| {
            let ids_str = ids.join(" ");
            let _ = navigator.push_with_query(
                &Route::BibliZapResults,
                &BibliZapResultsQuery { ids: ids_str },
            );
        })
    };

    let content = match fetch_status.deref() {
        PubmedFetchStatus::Loading => html! { <Spinner /> },
        PubmedFetchStatus::Error(e) => html! { <ErrorMessage msg={e.to_string()} /> },
        PubmedFetchStatus::Success(results) => html! {
            <PubMedResultsView results={results.clone()} on_run_snowball={on_run_snowball} />
        },
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
