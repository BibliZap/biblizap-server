use gloo_file::{callbacks::read_as_text, File};
use wasm_bindgen_futures::spawn_local;
use web_sys::{DragEvent, HtmlInputElement};
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

    let on_file = {
        let state = state.clone();
        let navigator = navigator.clone();
        let reader_task = reader_task.clone();
        Callback::from(move |file: File| {
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
                PageState::Idle => html! { <DropZone on_file={on_file} /> },
                PageState::Loading => html! { <LoadingState /> },
                PageState::Error(msg) => html! { <ErrorState {msg} on_file={on_file} /> },
            }}
        </div>
    }
}

#[function_component]
fn LoadingState() -> Html {
    html! {
        <div class="d-flex align-items-center gap-2 text-muted">
            <div class="spinner-border spinner-border-sm" role="status" />
            <span>{"Uploading bibliography\u{2026}"}</span>
        </div>
    }
}

#[derive(Clone, PartialEq, Properties)]
struct ErrorStateProps {
    msg: String,
    on_file: Callback<File>,
}

#[function_component]
fn ErrorState(props: &ErrorStateProps) -> Html {
    html! {
        <div class="d-flex flex-column gap-3">
            <div class="alert alert-danger mb-0" role="alert">
                <i class="bi bi-exclamation-triangle-fill me-2" />
                { &props.msg }
            </div>
            <DropZone on_file={props.on_file.clone()} />
        </div>
    }
}

#[derive(Clone, PartialEq, Properties)]
struct DropZoneProps {
    on_file: Callback<File>,
}

#[function_component]
fn DropZone(props: &DropZoneProps) -> Html {
    let is_dragging = use_state(|| false);

    let ondragover = {
        let is_dragging = is_dragging.clone();
        Callback::from(move |e: DragEvent| {
            e.prevent_default();
            is_dragging.set(true);
        })
    };

    let ondragleave = {
        let is_dragging = is_dragging.clone();
        Callback::from(move |_: DragEvent| {
            is_dragging.set(false);
        })
    };

    let ondrop = {
        let is_dragging = is_dragging.clone();
        let on_file = props.on_file.clone();
        Callback::from(move |e: DragEvent| {
            e.prevent_default();
            is_dragging.set(false);
            let file = e
                .data_transfer()
                .and_then(|dt| dt.files())
                .and_then(|fl| fl.get(0))
                .map(File::from);
            if let Some(file) = file {
                on_file.emit(file);
            }
        })
    };

    let onchange = {
        let on_file = props.on_file.clone();
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            let file = input.files().and_then(|f| f.get(0)).map(File::from);
            if let Some(file) = file {
                on_file.emit(file);
            }
        })
    };

    let border_class = if *is_dragging {
        "border-primary bg-primary-subtle"
    } else {
        "border-secondary-subtle"
    };

    html! {
        <label
            class={classes!("d-flex", "flex-column", "align-items-center", "justify-content-center",
                "border", "border-2", "rounded-3", "p-5", "w-100", "text-center", "gap-2",
                border_class)}
            style="border-style: dashed !important; cursor: pointer;"
            ondragover={ondragover}
            ondragleave={ondragleave}
            ondrop={ondrop}
        >
            <i class="bi bi-upload fs-3 text-secondary" />
            <span class="fw-medium">{"Click to upload or drag & drop"}</span>
            <small class="text-muted">{".ris, .nbib, .bzd"}</small>
            <input type="file" accept=".ris,.nbib,.bzd" hidden=true onchange={onchange} />
        </label>
    }
}
