use gloo_file::{callbacks::read_as_text, File};
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::common::{Route, SeedSelectionQuery};
use crate::search::denylist::{extract_dois, upload_denylist_to_backend};

#[derive(Clone, PartialEq)]
enum PageState {
    Idle,
    Loading,
    Error(String),
}

#[function_component]
pub fn SystematicReviewPage() -> Html {
    let navigator = use_navigator().unwrap();
    let state = use_state(|| PageState::Idle);
    let reader_task = use_mut_ref(|| None);

    let on_file_change = {
        let state = state.clone();
        let navigator = navigator.clone();
        let reader_task = reader_task.clone();
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            let file = match input.files().and_then(|f| f.get(0)) {
                Some(f) => File::from(f),
                None => return,
            };
            state.set(PageState::Loading);
            let state = state.clone();
            let navigator = navigator.clone();
            let task = read_as_text(&file, move |result| {
                let content = match result {
                    Ok(c) => c,
                    Err(_) => {
                        state.set(PageState::Error("Failed to read the file.".to_string()));
                        return;
                    }
                };
                let dois = match extract_dois(&content) {
                    Some(dois) if !dois.is_empty() => dois,
                    _ => {
                        state.set(PageState::Error(
                            "No DOIs found. Make sure the file is a valid .ris, .nbib, or .bzd file.".to_string(),
                        ));
                        return;
                    }
                };
                let state = state.clone();
                let navigator = navigator.clone();
                spawn_local(async move {
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
                            state.set(PageState::Error(format!("Upload failed: {e}")));
                        }
                    }
                });
            });
            *reader_task.borrow_mut() = Some(task);
        })
    };

    html! {
        <div>
            <h2 class="mb-1">{"Systematic Review"}</h2>
            <p class="text-muted mb-4">
                {"Upload your existing reference list to browse its articles and select which ones to use as seeds for BibliZap."}
            </p>
            { match (*state).clone() {
                PageState::Idle => html! { <UploadButton on_file_change={on_file_change} label="Upload bibliography" /> },
                PageState::Loading => html! {
                    <div class="d-flex align-items-center gap-2 text-muted">
                        <div class="spinner-border spinner-border-sm" role="status" />
                        <span>{"Uploading bibliography\u{2026}"}</span>
                    </div>
                },
                PageState::Error(msg) => html! {
                    <div class="d-flex flex-column gap-3">
                        <div class="alert alert-danger mb-0" role="alert">
                            <i class="bi bi-exclamation-triangle-fill me-2" />
                            { msg }
                        </div>
                        <UploadButton on_file_change={on_file_change} label="Try again" />
                    </div>
                },
            }}
        </div>
    }
}

/// Placeholder shown at `/seed-selection` until the seed selection page is implemented.
#[function_component]
pub fn SeedSelectionPage() -> Html {
    html! {
        <div class="text-muted">{"Seed selection — coming soon."}</div>
    }
}

#[derive(Clone, PartialEq, Properties)]
struct UploadButtonProps {
    on_file_change: Callback<Event>,
    label: &'static str,
}

#[function_component]
fn UploadButton(props: &UploadButtonProps) -> Html {
    html! {
        <label class="btn btn-primary align-self-start">
            <i class="bi bi-upload me-2" />
            { props.label }
            <input type="file" accept=".ris,.nbib,.bzd" hidden=true onchange={props.on_file_change.clone()} />
        </label>
    }
}
