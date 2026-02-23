use std::rc::Rc;
use std::{cell::RefCell, ops::Deref};

use gloo_console::log;
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
use common::{CurrentPage, Error};

/// The main application component.
/// Manages the current page state and dark mode state.
#[function_component(App)]
fn app() -> Html {
    let current_page = use_state(|| CurrentPage::BibliZapApp);
    let dark_mode = use_state(|| false);

    // Handle Biblitest token on mount
    {
        let current_page = current_page.clone();
        use_effect_with((), move |_| {
            // Only process token on the main app page
            if *current_page == CurrentPage::BibliZapApp {
                wasm_bindgen_futures::spawn_local(async move {
                    if let Err(e) = handle_biblitest_token().await {
                        log!("Failed to process Biblitest token: {:?}", e.to_string());
                    }
                });
            }
            || ()
        });
    }

    match dark_mode.deref() {
        true => gloo_utils::document_element()
            .set_attribute("data-bs-theme", "dark")
            .unwrap_or(()),
        false => gloo_utils::document_element()
            .set_attribute("data-bs-theme", "light")
            .unwrap_or(()),
    }

    let content = match current_page.deref() {
        CurrentPage::BibliZapApp => {
            html! {<BibliZapApp/>}
        }
        CurrentPage::HowItWorks => {
            html! {<HowItWorks/>}
        }
        CurrentPage::LegalInformation => {
            html! {<LegalInformation/>}
        }
        CurrentPage::Contact => {
            html! {<Contact/>}
        }
    };
    html! {
        <div class="d-flex flex-column min-vh-100">
            <NavBar current_page={current_page} dark_mode={dark_mode}/>
            <div class="container my-4">
                {content}
            </div>
            <Wall/>
        </div>
    }
}

/// Handles the Biblitest token from URL parameters.
/// Reads the token, sends it to /link endpoint, and removes it from the URL.
async fn handle_biblitest_token() -> Result<(), Error> {
    use gloo_utils::document;

    // Get current URL
    let url_str = document()
        .document_uri()
        .map_err(|e| Error::JsValueString(e.as_string().unwrap_or_default()))?;

    let url = url::Url::parse(&url_str)?;

    // Look for biblitest_token parameter
    let token = url
        .query_pairs()
        .find(|(k, _)| k == "biblitest_token")
        .map(|(_, v)| v.to_string());

    if let Some(token) = token {
        log!("Found Biblitest token, linking session...");

        // Send token to /link endpoint
        let mut api_url = url.clone();
        api_url.set_query(None);
        api_url.set_fragment(None);
        api_url.set_path("link");

        let response = gloo_net::http::Request::post(api_url.as_str())
            .json(&serde_json::json!({
                "biblitest_token": token
            }))?
            .send()
            .await?;

        if response.ok() {
            log!("Session linked successfully");

            // Remove token from URL using history API
            if let Some(window) = web_sys::window() {
                if let Some(history) = window.history().ok() {
                    let mut clean_url = url.clone();

                    // Remove biblitest_token from query params
                    let new_query: String = url
                        .query_pairs()
                        .filter(|(k, _)| k != "biblitest_token")
                        .map(|(k, v)| format!("{}={}", k, v))
                        .collect::<Vec<_>>()
                        .join("&");

                    if new_query.is_empty() {
                        clean_url.set_query(None);
                    } else {
                        clean_url.set_query(Some(&new_query));
                    }

                    let _ = history.replace_state_with_url(
                        &wasm_bindgen::JsValue::NULL,
                        "",
                        Some(clean_url.as_str()),
                    );
                }
            }
        } else {
            let error_text = response.text().await?;
            log!("Failed to link session: {}", error_text);
        }
    }

    Ok(())
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
                Ok(table) => results_status.set(ResultsStatus::Available(table)),
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
                <SnowballForm {on_submit_error} {on_requesting_results} {on_receiving_response}/>
            </div>
            <div class={results_class}>
                <ResultsContainer results_status={results_status.clone()}/>
            </div>
        </div>
    }
}

/// Entry point for the Yew frontend application.
fn main() {
    yew::Renderer::<App>::new().render();
}
