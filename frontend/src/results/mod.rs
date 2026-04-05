use std::cell::RefCell;
use std::collections::HashSet;
use std::ops::Deref;
use std::rc::Rc;

use yew::prelude::*;
use yew_router::prelude::*;

use crate::common::{OutputMaxSize, SearchFor};

pub mod article;
pub use article::Article;

mod download;
use download::*;

mod item;
use item::Item;

mod organize;
use organize::*;

use crate::common::Error;

/// Component for displaying a loading spinner.
#[function_component]
pub fn Spinner() -> Html {
    html! {
        <div class="container-fluid mt-5">
            <div class="d-flex justify-content-center">
                <div class="spinner-border" role="status" style="width: 5rem; height: 5rem; margin-bottom: 50px;">
                    <span class="visually-hidden">{"Loading..."}</span>
                </div>
            </div>
        </div>
    }
}

/// Properties for the Results (Table) component.
#[derive(Clone, PartialEq, Properties)]
pub struct ResultsProps {
    articles: Rc<RefCell<Vec<Article>>>,
    on_run_snowball: Callback<Vec<String>>,
}

/// Component for displaying the search results in a table.
#[function_component]
pub fn Results(props: &ResultsProps) -> Html {
    let selected_articles = use_state(|| HashSet::<String>::new());

    let update_selected = {
        let selected_articles = selected_articles.clone();
        Callback::from(move |element: (String, bool)| {
            let mut current_selected = (*selected_articles).clone();
            if element.1 {
                current_selected.insert(element.0);
            } else {
                current_selected.remove(&element.0);
            }
            selected_articles.set(current_selected);
        })
    };

    let articles = props.articles.to_owned();
    let global_filter = use_state(|| "".to_string());

    let articles_to_display = articles
        .deref()
        .borrow()
        .iter()
        .filter(|a| a.matches_global(&global_filter))
        .cloned()
        .collect::<Vec<_>>();

    let on_rerun_click = {
        let articles = props.articles.clone();
        let selected_articles = selected_articles.clone();
        let on_run_snowball = props.on_run_snowball.clone();
        Callback::from(move |_: MouseEvent| {
            let articles_to_download = get_articles_to_download(&articles, &selected_articles);
            let ids: Vec<String> = articles_to_download
                .iter()
                .filter_map(|a| a.doi.clone())
                .collect();
            on_run_snowball.emit(ids);
        })
    };

    let on_copy_click = {
        let articles = props.articles.clone();
        let selected_articles = selected_articles.clone();
        Callback::from(move |_: MouseEvent| {
            let articles = get_articles_to_download(&articles, &selected_articles);
            let mut ids: Vec<String> = articles.into_iter().filter_map(|a| a.doi).collect();
            ids.sort();
            ids.dedup();
            let ids_str = ids.join("\n");

            wasm_bindgen_futures::spawn_local(async move {
                let window = web_sys::window().expect("Window should exist");
                let navigator = window.navigator();
                let clipboard = navigator.clipboard();

                let promise = clipboard.write_text(&ids_str);
                let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
            });
        })
    };

    let display_limit = use_state(|| 20usize);

    let articles_slice =
        &articles_to_display[0..std::cmp::min(*display_limit, articles_to_display.len())];

    let trigger_update = use_force_update();
    let redraw_table = {
        let trigger_update = trigger_update.clone();
        Callback::from(move |_: ()| {
            trigger_update.force_update();
        })
    };

    let update_display_limit = {
        let display_limit = display_limit.clone();
        Callback::from(move |additional_limit: usize| {
            display_limit.set(*display_limit + additional_limit);
        })
    };

    html! {
        <div id="table" class="container-fluid py-4">
            <div class="row mb-4 align-items-center bg-light-subtle p-3 rounded border">
                <GlobalFilter on_name_entry={
                    Callback::from(move |value: String| {
                        global_filter.set(value);
                    })
                } />
                <SortButtons articles={props.articles.clone()} redraw_table={redraw_table.clone()} />
            </div>

            // Modern List View
            <div class="result-items-list">
                { articles_slice.iter().enumerate().map(|(i, article)| html!{
                    <Item
                        key={article.doi.clone().unwrap_or_else(|| i.to_string())}
                        article={article.clone()}
                        index={i}
                        update_selected={update_selected.clone()}
                        selected_articles={(*selected_articles).clone()}
                    />
                } ).collect::<Html>() }
            </div>

            <LoadMoreArticlesButton
                n_articles={articles_to_display.len()}
                display_limit={*display_limit.clone()}
                update_display_limit={update_display_limit.clone()}
            />

            <div class="mt-5 p-3 bg-light border rounded d-flex gap-3 align-items-center flex-wrap shadow-sm">
                <h5>{
                    if selected_articles.is_empty() {
                        "Download all articles:".to_string()
                    } else {
                        format!("Selected ({}) actions:", selected_articles.len())
                    }
                }</h5>
                {if !selected_articles.is_empty() {
                    html! {
                        <>
                            <button class="btn btn-primary" onclick={on_rerun_click}>
                                <i class="bi bi-search"></i> {" Rerun BibliZap"}
                            </button>
                            <button class="btn btn-outline-secondary" onclick={on_copy_click} title="Copy DOIs/PMIDs to clipboard">
                                <i class="bi bi-clipboard"></i> {" Copy"}
                            </button>
                            <div class="vr mx-2"></div>
                        </>
                    }
                } else {
                    html! {}
                }}
                    <DownloadButtons articles={articles.clone()} selected_articles={(*selected_articles).clone()} />
                </div>
        </div>
    }
}

// The advanced headers are no longer implemented in the simple control bar,
// but we leave Error handling component here.

/// Fetches BibliZap snowball results for a list of IDs from the backend API.
/// Expert params default to `Limit(100)`, depth 2, and `Both` when `None`.
pub async fn run_snowball_with_ids(
    ids: &[String],
    depth: Option<u8>,
    output_max_size: Option<&OutputMaxSize>,
    search_for: Option<&SearchFor>,
) -> Result<Rc<RefCell<Vec<Article>>>, Error> {
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
    api_url.set_path("api");

    let body = serde_json::json!({
        "output_max_size": output_max_size.unwrap_or(&OutputMaxSize::Limit(100)),
        "depth": depth.unwrap_or(2),
        "input_id_list": ids,
        "search_for": search_for.unwrap_or(&SearchFor::Both)
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

    let value = serde_json::from_str::<serde_json::Value>(&result_text)?;
    let mut articles = serde_json::from_value::<Vec<Article>>(value)?;

    articles.sort_by_key(|article| std::cmp::Reverse(article.score.unwrap_or_default()));

    Ok(Rc::new(RefCell::new(articles)))
}

fn location_to_query(
    location: &Option<Location>,
) -> Result<crate::common::BibliZapResultsQuery, Error> {
    location
        .as_ref()
        .and_then(|l| l.query::<crate::common::BibliZapResultsQuery>().ok())
        .ok_or_else(|| Error::JsValueString("Missing query parameters".to_string()))
}

enum FetchStatus {
    Loading,
    Success(Rc<RefCell<Vec<Article>>>),
    Error(Error),
}

/// The BibliZap results page.
/// Reads `?ids=` (and optional expert params) from the URL, fetches results on mount,
/// and renders the results table.
/// If navigated to via the search form (history state = `FromSearch(true)`), the search
/// bar starts centred and rises to the top via a CSS transition on the next paint.
/// On direct/bookmarked access it starts at the top immediately.
#[function_component]
pub fn BibliZapResults() -> Html {
    use crate::common::{BibliZapResultsQuery, FormPosition, Route};
    use crate::search::{AdvancedParams, BiblizapSearchBar};

    let location = use_location();
    let navigator = use_navigator().unwrap();

    let Ok(query) = location_to_query(&location) else {
        navigator.replace(&Route::BibliZapSearch);
        return html! {};
    };

    let ids: Vec<String> = query
        .ids
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();
    if ids.is_empty() {
        navigator.replace(&Route::BibliZapSearch);
        return html! {};
    }

    // Only animate when explicitly navigated from the search form.
    let form_position = location
        .as_ref()
        .and_then(|l| l.state::<FormPosition>())
        .map(|s| s.deref().to_owned())
        .unwrap_or_default();

    let form_class = form_position.get_class();
    gloo_console::log!(format!("Form position: {:#?}", form_position));

    let fetch_status: UseStateHandle<FetchStatus> = use_state(|| FetchStatus::Loading);
    {
        let fetch_status = fetch_status.clone();
        let ids = ids.clone();
        let depth = query.depth;
        let output_max_size = query.output_max_size.clone();
        let search_for = query.search_for.clone();
        // Depend on the full query string so re-navigating with different params re-fetches.
        let query_key = format!(
            "{} {:?} {:?} {:?}",
            ids.join(" "),
            depth,
            output_max_size,
            search_for
        );
        use_effect_with(query_key, move |_| {
            fetch_status.set(FetchStatus::Loading);
            wasm_bindgen_futures::spawn_local(async move {
                let result = run_snowball_with_ids(
                    &ids,
                    depth,
                    output_max_size.as_ref(),
                    search_for.as_ref(),
                )
                .await;
                match result {
                    Ok(articles) => fetch_status.set(FetchStatus::Success(articles)),
                    Err(e) => fetch_status.set(FetchStatus::Error(e)),
                }
            });
            || ()
        });
    }

    // Preserve current page's expert params when re-running on a selection.
    let on_run_snowball = {
        let navigator = navigator.clone();
        let depth = query.depth;
        let output_max_size = query.output_max_size.clone();
        let search_for = query.search_for.clone();
        Callback::from(move |ids: Vec<String>| {
            let ids_str = ids.join(" ");
            let _ = navigator.push_with_query(
                &Route::BibliZapResults,
                &BibliZapResultsQuery {
                    ids: ids_str,
                    depth,
                    output_max_size: output_max_size.clone(),
                    search_for: search_for.clone(),
                },
            );
        })
    };

    let content = match fetch_status.deref() {
        FetchStatus::Loading => html! { <Spinner /> },
        FetchStatus::Error(msg) => html! { <ErrorMessage msg={msg.to_string()} /> },
        FetchStatus::Success(articles) => html! {
            <Results
                articles={articles}
                on_run_snowball={on_run_snowball}
            />
        },
    };

    html! {
        <div>
            <div class={form_class}>
                <BiblizapSearchBar
                    position={FormPosition::Top}
                    value={ids.join(" ")}
                    advanced={Some(AdvancedParams::from(&query))}
                />
            </div>
            <div class="results-fade-in">
                {content}
            </div>
        </div>
    }
}

/// Properties for the Error component.
#[derive(Clone, PartialEq, Properties)]
pub struct ErrorProps {
    pub msg: AttrValue,
}

/// Component for displaying an error message.
#[function_component]
pub fn ErrorMessage(props: &ErrorProps) -> Html {
    html! {
        <div class="container-fluid">
            <div class="alert alert-danger" role="alert">
                {props.msg.to_owned()}
            </div>
        </div>
    }
}
