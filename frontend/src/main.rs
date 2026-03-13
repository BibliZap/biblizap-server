use std::rc::Rc;
use std::{cell::RefCell, ops::Deref};

use yew::prelude::*;

mod legal;
use legal::*;

mod results;
use results::*;

mod navbar;
use navbar::*;

mod wall;
use wall::*;

mod form;
use form::SnowballForm;

mod common;
use common::Error;

use yew_router::prelude::*;

use crate::common::Route;
use crate::results::pubmed::PubmedSearchResult;

/// The main application component.
/// Manages the current page state and dark mode state.
#[function_component(App)]
fn app() -> Html {
    let dark_mode = use_state(|| false);

    match dark_mode.deref() {
        true => gloo_utils::document_element()
            .set_attribute("data-bs-theme", "dark")
            .unwrap_or(()),
        false => gloo_utils::document_element()
            .set_attribute("data-bs-theme", "light")
            .unwrap_or(()),
    }

    html! {
        <BrowserRouter>
        <div class="d-flex flex-column min-vh-100">
            <NavBar dark_mode={dark_mode}/>
            <div class="container my-4">
                    <Switch<Route> render={switch} />
            </div>
            <Wall/>
        </div>
        </BrowserRouter>
    }
}

/// The main BibliZap application page component.
/// Contains the search form and the results container.
/// Manages the state of the search results.
#[function_component(BibliZapApp)]
fn app() -> Html {
    let results_status = use_state(|| ResultsStatus::NotRequested);
    let on_receiving_response = {
        let results_status = results_status.clone();
        Callback::from(move |table: Result<Rc<RefCell<Vec<Article>>>, Error>| {
            match table {
                Ok(table) => results_status.set(ResultsStatus::BiblizapResults(table)),
                Err(error) => results_status.set(ResultsStatus::RequestError(error.to_string())),
            };
        })
    };
    let on_requesting_results = {
        let results_status = results_status.clone();
        Callback::from(move |_: ()| {
            results_status.set(ResultsStatus::Requested);
        })
    };

    let on_submit_error = {
        let results_status = results_status.clone();
        Callback::from(move |error: common::Error| {
            results_status.set(ResultsStatus::RequestError(error.to_string()))
        })
    };

    let on_pubmed_results = {
        let results_status = results_status.clone();
        Callback::from(move |pubmed_results: Vec<PubmedSearchResult>| {
            if pubmed_results.is_empty() {
                results_status.set(ResultsStatus::RequestError(
                    "No results found on PubMed for your query.".to_string(),
                ));
            } else {
                results_status.set(ResultsStatus::PubMedResults(pubmed_results));
            }
        })
    };

    // Callback when user selects articles from PubMed results and clicks "Run BibliZap"
    let on_run_snowball = {
        let results_status = results_status.clone();
        let on_receiving_response = on_receiving_response.clone();
        Callback::from(move |ids: Vec<String>| {
            if ids.is_empty() {
                return;
            }
            results_status.set(ResultsStatus::Requested);

            let on_receiving_response = on_receiving_response.clone();
            let ids = ids.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let result = run_snowball_with_ids(&ids).await;
                on_receiving_response.emit(result);
            });
        })
    };

    let form_class = match results_status.deref() {
        ResultsStatus::NotRequested => "form-container-centered",
        ResultsStatus::RequestError(_) => "form-container-centered",
        _ => "form-container-top",
    };

    let results_class = match results_status.deref() {
        ResultsStatus::NotRequested => "",
        ResultsStatus::RequestError(_) => "",
        _ => "results-fade-in",
    };

    html! {
        <div>
            <div class={form_class}>
                <SnowballForm {on_submit_error} {on_requesting_results} {on_receiving_response} {on_pubmed_results}/>
            </div>
            <div class={results_class}>
                <ResultsContainer results_status={results_status.clone()} on_run_snowball={on_run_snowball.clone()} />
            </div>
        </div>
    }
}

/// Runs the snowball search with a list of article IDs (DOIs or PMIDs).
/// This is called when the user selects articles from PubMed keyword search results.
async fn run_snowball_with_ids(ids: &[String]) -> Result<Rc<RefCell<Vec<Article>>>, Error> {
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

fn switch(routes: Route) -> Html {
    match routes {
        Route::BibliZapApp => html! { <BibliZapApp/> },
        Route::HowItWorks => html! { <HowItWorks /> },
        Route::Contact => html! { <Contact /> },
        Route::LegalInformation => html! { <LegalInformation /> },
        Route::NotFound => html! { <BibliZapApp/> },
    }
}

/// Entry point for the Yew frontend application.
fn main() {
    yew::Renderer::<App>::new().render();
}
