use std::cell::RefCell;
use std::collections::HashSet;
use std::ops::Deref;
use std::rc::Rc;

use yew::prelude::*;
use yew_router::prelude::*;

pub mod article;
pub use article::Article;

mod filter;
use filter::Filters;

mod download;
use download::*;

mod item;
use item::Item;

use crate::common::Error;

/// Enum representing the sort state of a column.
#[derive(Clone, Copy, PartialEq, Debug)]
enum SortState {
    None,
    Ascending,
    Descending,
}

impl SortState {
    fn next(&self) -> Self {
        match self {
            SortState::None => SortState::Ascending,
            SortState::Ascending => SortState::Descending,
            SortState::Descending => SortState::None,
        }
    }

    fn icon(&self) -> &'static str {
        match self {
            SortState::None => "bi-chevron-expand",
            SortState::Ascending => "bi-chevron-up",
            SortState::Descending => "bi-chevron-down",
        }
    }
}

/// Component for displaying a loading spinner.
#[function_component(Spinner)]
pub fn spinner() -> Html {
    html! {
        <div class="container-fluid">
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
pub struct TableProps {
    articles: Rc<RefCell<Vec<Article>>>,
    on_run_snowball: Callback<Vec<String>>,
}

/// Component for displaying the search results in a table.
#[function_component(Results)]
pub fn results(props: &TableProps) -> Html {
    let selected_articles = use_state(|| Rc::new(RefCell::new(HashSet::<String>::new())));

    let update_selected = {
        let selected_articles = selected_articles.clone();
        Callback::from(move |element: (String, bool)| {
            let current_selected = (*selected_articles).clone();
            if element.1 {
                current_selected.borrow_mut().insert(element.0);
            } else {
                current_selected.borrow_mut().remove(&element.0);
            }
            selected_articles.set(current_selected);
        })
    };

    let articles = props.articles.to_owned();
    let global_filter = use_state(|| "".to_string());
    let filters = use_mut_ref(Filters::default);
    let filters = use_state(|| filters);

    // Track sort state for each sortable column
    let sort_year = use_state(|| SortState::None);
    let sort_citations = use_state(|| SortState::None);
    let sort_score = use_state(|| SortState::None);

    let articles_to_display = articles
        .deref()
        .borrow()
        .iter()
        .filter(|a| a.matches_global(&global_filter))
        .filter(|a| a.matches(&filters.deref().borrow()))
        .cloned()
        .collect::<Vec<_>>();

    // Helper function to get articles to download
    let get_articles_to_download = {
        let articles = articles.clone();
        let selected_articles = selected_articles.clone();
        move || -> Vec<Article> {
            if selected_articles.borrow().is_empty() {
                // If nothing selected, return all articles
                articles.deref().borrow().clone()
            } else {
                // Return only selected articles
                articles
                    .deref()
                    .borrow()
                    .iter()
                    .filter(|article| {
                        article
                            .doi
                            .as_ref()
                            .map(|doi| selected_articles.borrow().contains(doi))
                            .unwrap_or(false)
                    })
                    .cloned()
                    .collect()
            }
        }
    };

    let on_excel_download_click = {
        let get_articles = get_articles_to_download.clone();
        let articles = articles.clone();
        Callback::from(move |_: MouseEvent| {
            let articles_to_download = get_articles();
            let bytes = to_excel(&articles_to_download).unwrap();
            let timestamp = chrono::Local::now().to_rfc3339();
            let suffix = if articles_to_download.len() == articles.deref().borrow().len() {
                "all"
            } else {
                "selected"
            };

            match download_bytes_as_file(&bytes, &format!("BibliZap-{suffix}-{timestamp}.xlsx")) {
                Ok(_) => (),
                Err(error) => {
                    gloo_console::log!(format!("{error}"));
                }
            }
        })
    };

    let on_ris_download_click = {
        let get_articles = get_articles_to_download.clone();
        let articles = articles.clone();
        Callback::from(move |_: MouseEvent| {
            let articles_to_download = get_articles();
            let bytes = to_ris(&articles_to_download).unwrap();
            let timestamp = chrono::Local::now().to_rfc3339();
            let suffix = if articles_to_download.len() == articles.deref().borrow().len() {
                "all"
            } else {
                "selected"
            };

            match download_bytes_as_file(&bytes, &format!("BibliZap-{suffix}-{timestamp}.ris")) {
                Ok(_) => (),
                Err(error) => {
                    gloo_console::log!(format!("{error}"));
                }
            }
        })
    };

    let on_bibtex_download_click = {
        let get_articles = get_articles_to_download.clone();
        let articles = articles.clone();
        Callback::from(move |_: MouseEvent| {
            let articles_to_download = get_articles();
            let bytes = to_bibtex(&articles_to_download).unwrap();
            let timestamp = chrono::Local::now().to_rfc3339();
            let suffix = if articles_to_download.len() == articles.deref().borrow().len() {
                "all"
            } else {
                "selected"
            };

            match download_bytes_as_file(&bytes, &format!("BibliZap-{suffix}-{timestamp}.bib")) {
                Ok(_) => (),
                Err(error) => {
                    gloo_console::log!(format!("{error}"));
                }
            }
        })
    };

    let on_rerun_click = {
        let get_articles = get_articles_to_download.clone();
        let on_run_snowball = props.on_run_snowball.clone();
        Callback::from(move |_: MouseEvent| {
            let articles_to_download = get_articles();
            let ids: Vec<String> = articles_to_download
                .iter()
                .filter_map(|a| a.doi.clone())
                .collect();
            on_run_snowball.emit(ids);
        })
    };

    let on_copy_click = {
        let get_articles = get_articles_to_download.clone();
        Callback::from(move |_: MouseEvent| {
            let articles = get_articles();
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

    let on_load_more = {
        let display_limit = display_limit.clone();
        Callback::from(move |e: MouseEvent| {
            e.prevent_default();

            // Blur the button to prevent the browser from automatically scrolling
            // down to keep the focused button in the viewport.
            use wasm_bindgen::JsCast;
            if let Some(target) = e.target() {
                if let Ok(element) = target.dyn_into::<web_sys::HtmlElement>() {
                    let _ = element.blur();
                }
            }

            display_limit.set(*display_limit + 20);
        })
    };

    html! {
        <div id="table" class="container-fluid py-4">
            <div class="row mb-4 align-items-center bg-light p-3 rounded border">
                <div class="col-md-5 mb-3 mb-md-0">
                    <div class="input-group">
                        <span class="input-group-text bg-white"><i class="bi bi-search"></i></span>
                        <input type="text" class="form-control" placeholder="Search across all fields..." oninput={
                            let filter = global_filter.clone();
                            Callback::from(move |e: InputEvent| {
                                let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                filter.set(input.value());
                            })
                        } />
                    </div>
                </div>
                <div class="col-md-7 d-flex justify-content-md-end gap-2 flex-wrap">
                    <span class="align-self-center fw-semibold text-secondary me-2">{"Sort by:"}</span>
                    <button class={classes!("btn", "btn-sm", if *sort_year != SortState::None { "btn-primary" } else { "btn-outline-secondary" })} onclick={
                        let articles = props.articles.clone();
                        let redraw_table = redraw_table.clone();
                        let sort_year = sort_year.clone();
                        Callback::from(move |_: MouseEvent| {
                            let new_state = (*sort_year).next();
                            sort_year.set(new_state);
                            let mut ref_vec = articles.deref().borrow_mut();
                            match new_state {
                                SortState::None => ref_vec.sort_by_key(|a| std::cmp::Reverse(a.score.clone().unwrap_or_default())),
                                SortState::Ascending => ref_vec.sort_by_key(|a| a.year_published.clone().unwrap_or_default()),
                                SortState::Descending => ref_vec.sort_by_key(|a| std::cmp::Reverse(a.year_published.clone().unwrap_or_default())),
                            }
                            redraw_table.emit(());
                        })
                    }>
                        <i class="bi bi-calendar me-1"></i> {"Year"} {if *sort_year != SortState::None { html! {<i class={classes!("bi", sort_year.icon())}></i>} } else { html!{} }}
                    </button>
                    <button class={classes!("btn", "btn-sm", if *sort_citations != SortState::None { "btn-primary" } else { "btn-outline-secondary" })} onclick={
                        let articles = props.articles.clone();
                        let redraw_table = redraw_table.clone();
                        let sort_citations = sort_citations.clone();
                        Callback::from(move |_: MouseEvent| {
                            let new_state = (*sort_citations).next();
                            sort_citations.set(new_state);
                            let mut ref_vec = articles.deref().borrow_mut();
                            match new_state {
                                SortState::None => ref_vec.sort_by_key(|a| std::cmp::Reverse(a.score.clone().unwrap_or_default())),
                                SortState::Ascending => ref_vec.sort_by_key(|a| a.citations.clone().unwrap_or_default()),
                                SortState::Descending => ref_vec.sort_by_key(|a| std::cmp::Reverse(a.citations.clone().unwrap_or_default())),
                            }
                            redraw_table.emit(());
                        })
                    }>
                        <i class="bi bi-quote me-1"></i> {"Citations"} {if *sort_citations != SortState::None { html! {<i class={classes!("bi", sort_citations.icon())}></i>} } else { html!{} }}
                    </button>
                    <button class={classes!("btn", "btn-sm", if *sort_score != SortState::None { "btn-primary" } else { "btn-outline-secondary" })} onclick={
                        let articles = props.articles.clone();
                        let redraw_table = redraw_table.clone();
                        let sort_score = sort_score.clone();
                        Callback::from(move |_: MouseEvent| {
                            let new_state = (*sort_score).next();
                            sort_score.set(new_state);
                            let mut ref_vec = articles.deref().borrow_mut();
                            match new_state {
                                SortState::None => ref_vec.sort_by_key(|a| std::cmp::Reverse(a.score.clone().unwrap_or_default())),
                                SortState::Ascending => ref_vec.sort_by_key(|a| a.score.clone().unwrap_or_default()),
                                SortState::Descending => ref_vec.sort_by_key(|a| std::cmp::Reverse(a.score.clone().unwrap_or_default())),
                            }
                            redraw_table.emit(());
                        })
                    }>
                        <i class="bi bi-star-fill me-1"></i> {"Score"} {if *sort_score != SortState::None { html! {<i class={classes!("bi", sort_score.icon())}></i>} } else { html!{} }}
                    </button>
                </div>
            </div>

            // Modern List View
            <div class="mb-4">
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

            {if *display_limit < articles_to_display.len() {
                html! {
                    <div class="d-flex justify-content-center my-4">
                        <button class="btn btn-outline-primary rounded-pill px-4 py-2 fw-semibold" onclick={on_load_more}>
                            {"Load More Articles..."}
                        </button>
                    </div>
                }
            } else {
                html! {
                    <div class="text-center text-muted my-4 py-3 border-top">
                        <small>{"All "}{articles_to_display.len()}{" articles displayed."}</small>
                    </div>
                }
            }}

            <div class="mt-5 p-3 bg-light border rounded d-flex gap-3 align-items-center flex-wrap shadow-sm">
                <h5>{
                    if selected_articles.borrow().is_empty() {
                        "Download all articles:".to_string()
                    } else {
                        format!("Selected ({}) actions:", selected_articles.borrow().len())
                    }
                }</h5>
                {if !selected_articles.borrow().is_empty() {
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
                <DownloadButton onclick={on_excel_download_click} label="Excel"/>
                <DownloadButton onclick={on_ris_download_click} label="RIS"/>
                <DownloadButton onclick={on_bibtex_download_click} label="BibTeX"/>
            </div>
        </div>
    }
}

// The advanced headers are no longer implemented in the simple control bar,
// but we leave Error handling component here.

/// Fetches BibliZap snowball results for a list of IDs from the backend API.
pub async fn run_snowball_with_ids(ids: &[String]) -> Result<Rc<RefCell<Vec<Article>>>, Error> {
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
        "output_max_size": "100",
        "depth": 2,
        "input_id_list": ids,
        "search_for": "Both"
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

fn location_to_ids(location: &Option<Location>) -> Result<Vec<String>, Error> {
    let ids_string = location
        .as_ref()
        .and_then(|l| l.query::<crate::common::BibliZapResultsQuery>().ok())
        .ok_or_else(|| Error::JsValueString("Missing query parameters".to_string()))?
        .ids;

    let ids: Vec<String> = ids_string
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();

    if ids.is_empty() {
        return Err(Error::NoValidIds);
    }
    Ok(ids)
}

fn form_class_from_location(location: &Option<Location>) -> &'static str {
    let from_search = location
        .as_ref()
        .and_then(|l| l.state::<crate::common::FromSearch>())
        .is_some();

    if from_search {
        "form-container-centered"
    } else {
        "form-container-top"
    }
}

enum FetchStatus {
    Loading,
    Success(Rc<RefCell<Vec<Article>>>),
    Error(Error),
}

/// The BibliZap results page.
/// Reads `?ids=` from the URL, fetches results on mount, and renders the results table.
/// If navigated to via the search form (history state = `FromSearch`), the search bar
/// animates up from the centre; on direct/bookmarked access it starts at the top.
#[function_component(BibliZapResults)]
pub fn biblizap_results() -> Html {
    use crate::common::{BibliZapResultsQuery, Route};
    use crate::search::SnowballForm;

    let location = use_location();
    let navigator = use_navigator().unwrap();

    let ids: Vec<String> = location_to_ids(&location).unwrap_or_else(|_| {
        navigator.replace(&Route::BibliZapSearch);
        vec![]
    });

    let form_class = form_class_from_location(&location);

    let fetch_status: UseStateHandle<FetchStatus> = use_state(|| FetchStatus::Loading);

    {
        let fetch_status = fetch_status.clone();
        let ids = ids.clone();
        use_effect_with(location, move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                let result = run_snowball_with_ids(&ids).await;
                match result {
                    Ok(articles) => fetch_status.set(FetchStatus::Success(articles)),
                    Err(e) => fetch_status.set(FetchStatus::Error(e)),
                }
            });
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
                <SnowballForm />
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
    msg: AttrValue,
}

/// Component for displaying an error message.
#[function_component(ErrorMessage)]
pub fn error(props: &ErrorProps) -> Html {
    html! {
        <div class="container-fluid">
            <div class="alert alert-danger" role="alert">
                {props.msg.to_owned()}
            </div>
        </div>
    }
}
