use std::collections::HashSet;

use gloo_net::http::Request;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::common::{BibliZapResultsQuery, Route, SeedSelectionQuery};
use crate::results::{Article, Item, Spinner};

#[derive(Clone, PartialEq)]
enum LoadState {
    Loading,
    Loaded(Vec<Article>),
    Error(String),
}

/// Self-contained seed picker: fetches enriched articles for `bibliography_hash`
/// and renders the selectable list with the "Run BibliZap" button.
/// Embeddable in any page that already has a bibliography hash.
#[derive(Clone, PartialEq, Properties)]
pub struct SeedPickerProps {
    pub bibliography_hash: String,
}

#[function_component]
pub fn SeedPicker(props: &SeedPickerProps) -> Html {
    let load_state = use_state(|| LoadState::Loading);

    {
        let load_state = load_state.clone();
        let hash = props.bibliography_hash.clone();
        use_effect_with(hash, move |hash| {
            let hash = hash.clone();
            let load_state = load_state.clone();
            if hash.is_empty() {
                load_state.set(LoadState::Error(
                    "No bibliography hash provided.".to_string(),
                ));
            } else {
                spawn_local(async move {
                    let url = format!("/api/corpus/enrich/{}", hash);
                    match Request::get(&url).send().await {
                        Ok(resp) if resp.ok() => match resp.json::<Vec<Article>>().await {
                            Ok(articles) => load_state.set(LoadState::Loaded(articles)),
                            Err(e) => load_state
                                .set(LoadState::Error(format!("Failed to parse response: {e}"))),
                        },
                        Ok(resp) => load_state.set(LoadState::Error(format!(
                            "Server error: HTTP {}",
                            resp.status()
                        ))),
                        Err(e) => load_state.set(LoadState::Error(format!("Request failed: {e}"))),
                    }
                });
            }
            || ()
        });
    }

    match (*load_state).clone() {
        LoadState::Loading => html! { <Spinner /> },
        LoadState::Error(msg) => html! { <SeedSelectionError {msg} /> },
        LoadState::Loaded(articles) => html! {
            <SeedSelectionLoaded {articles} bibliography_hash={props.bibliography_hash.clone()} />
        },
    }
}

#[function_component]
pub fn SeedSelectionPage() -> Html {
    let location = use_location().unwrap();

    let bibliography_hash = location
        .query::<SeedSelectionQuery>()
        .ok()
        .map(|q| q.bibliography)
        .unwrap_or_default();

    html! { <SeedPicker {bibliography_hash} /> }
}

#[derive(Clone, PartialEq, Properties)]
struct SeedSelectionErrorProps {
    msg: String,
}

#[function_component]
fn SeedSelectionError(props: &SeedSelectionErrorProps) -> Html {
    html! {
        <div class="alert alert-danger" role="alert">
            <i class="bi bi-exclamation-triangle-fill me-2" />
            { &props.msg }
        </div>
    }
}

#[derive(Clone, PartialEq, Properties)]
struct SeedSelectionLoadedProps {
    articles: Vec<Article>,
    bibliography_hash: String,
}

#[function_component]
fn SeedSelectionLoaded(props: &SeedSelectionLoadedProps) -> Html {
    let navigator = use_navigator().unwrap();
    let selected = use_state(|| HashSet::<String>::new());

    let n_total = props.articles.len();
    let n_selected = selected.len();

    let run_label = match n_selected {
        0 => "Run BibliZap".to_string(),
        1 => "Run BibliZap with 1 seed".to_string(),
        n => format!("Run BibliZap with {} seeds", n),
    };

    let on_run = {
        let selected = selected.clone();
        let hash = props.bibliography_hash.clone();
        Callback::from(move |_: MouseEvent| {
            let ids_str = (*selected).iter().cloned().collect::<Vec<_>>().join(" ");
            if ids_str.is_empty() {
                return;
            }
            let _ = navigator.push_with_query(
                &Route::BibliZapResults,
                &BibliZapResultsQuery {
                    ids: ids_str,
                    depth: None,
                    output_max_size: None,
                    search_for: None,
                    denylist_hash: Some(hash.clone()),
                },
            );
        })
    };

    let update_selected = {
        let selected = selected.clone();
        Callback::from(move |(doi, checked): (String, bool)| {
            let mut s = (*selected).clone();
            if checked {
                s.insert(doi);
            } else {
                s.remove(&doi);
            }
            selected.set(s);
        })
    };

    let select_all = {
        let selected = selected.clone();
        let articles = props.articles.clone();
        Callback::from(move |_: MouseEvent| {
            let mut s = HashSet::new();
            for article in &articles {
                if let Some(id) = article.id() {
                    s.insert(id);
                }
            }
            selected.set(s);
        })
    };

    let clear_all = {
        let selected = selected.clone();
        Callback::from(move |_: MouseEvent| {
            selected.set(HashSet::new());
        })
    };

    html! {
        <div>
            <div class="d-flex justify-content-between align-items-center mb-4 gap-3 flex-wrap">
                <div>
                    <h2 class="mb-0">{"Select Seeds"}</h2>
                    <p class="text-muted mb-0 small">
                        { format!("{n_total} article{}", if n_total == 1 { "" } else { "s" }) }
                        { if n_selected > 0 { format!(" · {n_selected} selected") } else { String::new() } }
                    </p>
                </div>
                <div class="d-flex gap-2">
                    <button class="btn btn-outline-secondary btn-sm" onclick={select_all.clone()}>
                        <i class="bi bi-check-all me-1" />
                        {"Select All"}
                    </button>
                    <button class="btn btn-outline-secondary btn-sm" onclick={clear_all} disabled={n_selected == 0}>
                        <i class="bi bi-x me-1" />
                        {"Clear"}
                    </button>
                    <button class="btn btn-primary" disabled={n_selected == 0} onclick={on_run}>
                        <i class="bi bi-search me-2" />
                        { run_label }
                    </button>
                </div>
            </div>

            { if props.articles.is_empty() {
                html! {
                    <div class="alert alert-warning" role="alert">
                        {"No articles could be found from this bibliography. The DOIs may not be indexed in Lens.org."}
                    </div>
                }
            } else {
                html! {
                    <div class="d-flex flex-column gap-3">
                        { props.articles.iter().enumerate().map(|(index, article)| html! {
                            <Item
                                article={article.clone()}
                                {index}
                                update_selected={update_selected.clone()}
                                selected_articles={(*selected).clone()}
                            />
                        }).collect::<Html>() }
                    </div>
                }
            }}
        </div>
    }
}
