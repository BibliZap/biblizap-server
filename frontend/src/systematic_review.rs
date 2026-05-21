use gloo_file::{callbacks::read_as_text, File};
use wasm_bindgen_futures::spawn_local;
use web_sys::{DragEvent, HtmlInputElement};
use yew::prelude::*;
use yew_router::prelude::*;

use crate::common::{Route, SeedSelectionQuery};
use crate::search::denylist::{extract_dois, upload_denylist_to_backend};

#[function_component]
pub fn SystematicReviewPage() -> Html {
    let navigator = use_navigator().unwrap();
    let uploading = use_state(|| false);
    let error = use_state(|| Option::<String>::None);
    let bib_dois = use_state(|| Option::<Vec<String>>::None);
    let excl_dois = use_state(|| Vec::<String>::new());
    let excl_file_count = use_state(|| 0usize);
    let bib_reader = use_mut_ref(|| None);
    let excl_reader = use_mut_ref(|| None);

    let on_bib_file = {
        let error = error.clone();
        let bib_dois = bib_dois.clone();
        let bib_reader = bib_reader.clone();
        Callback::from(move |file: File| {
            error.set(None);
            let bib_dois = bib_dois.clone();
            let error = error.clone();
            let task = read_as_text(&file, move |result| {
                let content = match result {
                    Ok(c) => c,
                    Err(_) => {
                        error.set(Some("Failed to read the file.".to_string()));
                        return;
                    }
                };
                match extract_dois(&content) {
                    Some(dois) if !dois.is_empty() => bib_dois.set(Some(dois)),
                    _ => error.set(Some(
                        "No DOIs found. Make sure the file is a valid .ris, .nbib, or .bzd file."
                            .to_string(),
                    )),
                }
            });
            *bib_reader.borrow_mut() = Some(task);
        })
    };

    let on_excl_file = {
        let excl_dois = excl_dois.clone();
        let excl_file_count = excl_file_count.clone();
        let excl_reader = excl_reader.clone();
        Callback::from(move |file: File| {
            let excl_dois = excl_dois.clone();
            let excl_file_count = excl_file_count.clone();
            let task = read_as_text(&file, move |result| {
                let Ok(content) = result else { return };
                if let Some(new_dois) = extract_dois(&content) {
                    let mut current = (*excl_dois).clone();
                    current.extend(new_dois);
                    current.sort_unstable();
                    current.dedup();
                    excl_dois.set(current);
                    excl_file_count.set(*excl_file_count + 1);
                }
            });
            *excl_reader.borrow_mut() = Some(task);
        })
    };

    let on_continue = {
        let bib_dois = bib_dois.clone();
        let excl_dois = excl_dois.clone();
        let uploading = uploading.clone();
        let error = error.clone();
        let navigator = navigator.clone();
        Callback::from(move |_: MouseEvent| {
            let Some(bib) = (*bib_dois).clone() else {
                return;
            };
            let excl = (*excl_dois).clone();
            uploading.set(true);
            let error = error.clone();
            let uploading_c = uploading.clone();
            let navigator = navigator.clone();
            spawn_local(async move {
                let bib_hash = match upload_denylist_to_backend(bib).await {
                    Ok(h) => h,
                    Err(e) => {
                        error.set(Some(format!("Upload failed: {e}")));
                        uploading_c.set(false);
                        return;
                    }
                };
                let excl_hash = if excl.is_empty() {
                    None
                } else {
                    match upload_denylist_to_backend(excl).await {
                        Ok(h) => Some(hex::encode(h)),
                        Err(e) => {
                            error.set(Some(format!("Exclusion upload failed: {e}")));
                            uploading_c.set(false);
                            return;
                        }
                    }
                };
                let _ = navigator.push_with_query(
                    &Route::SeedSelection,
                    &SeedSelectionQuery {
                        bibliography: hex::encode(bib_hash),
                        denylist: excl_hash,
                    },
                );
            });
        })
    };

    let bib_doi_count = (*bib_dois).as_ref().map(|v| v.len());
    let excl_doi_count = (*excl_dois).len();
    let excl_fc = *excl_file_count;
    let is_uploading = *uploading;

    html! {
        <div>
            <h2 class="mb-1">{"Systematic Review"}</h2>
            <p class="text-muted mb-4">
                {"Upload your existing reference list to browse its articles and select seeds for BibliZap."}
            </p>
            if let Some(msg) = (*error).clone() {
                <div class="alert alert-danger mb-3" role="alert">
                    <i class="bi bi-exclamation-triangle-fill me-2" />
                    { msg }
                </div>
            }
            <div class="row g-3 mb-3">
                <div class="col-md-6">
                    <BibDropZone on_file={on_bib_file} doi_count={bib_doi_count} />
                </div>
                <div class="col-md-6">
                    <ExclDropZone on_file={on_excl_file} doi_count={excl_doi_count} file_count={excl_fc} />
                </div>
            </div>
            <div class="d-flex align-items-center gap-3">
                <button
                    class="btn btn-primary"
                    onclick={on_continue}
                    disabled={is_uploading || bib_doi_count.is_none()}
                >
                    if is_uploading {
                        <>
                            <span class="spinner-border spinner-border-sm me-2" role="status" />
                            {"Uploading\u{2026}"}
                        </>
                    } else {
                        <>
                            <i class="bi bi-arrow-right me-2" />
                            {"Continue"}
                        </>
                    }
                </button>
                if let Some(n_bib) = bib_doi_count {
                    <small class="text-muted">
                        { format!("{} article{} in bibliography", n_bib, if n_bib == 1 { "" } else { "s" }) }
                        if excl_doi_count > 0 {
                            { format!(" \u{00b7} {} to exclude", excl_doi_count) }
                        }
                    </small>
                }
            </div>
        </div>
    }
}

// ---- BibDropZone -------------------------------------------------------

#[derive(Clone, PartialEq, Properties)]
struct BibDropZoneProps {
    on_file: Callback<File>,
    /// None = no file yet; Some(n) = n DOIs parsed.
    doi_count: Option<usize>,
}

#[function_component]
fn BibDropZone(props: &BibDropZoneProps) -> Html {
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
        Callback::from(move |_: DragEvent| is_dragging.set(false))
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
            if let Some(f) = file {
                on_file.emit(f);
            }
        })
    };
    let onchange = {
        let on_file = props.on_file.clone();
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            let file = input.files().and_then(|f| f.get(0)).map(File::from);
            if let Some(f) = file {
                on_file.emit(f);
            }
        })
    };

    let (border_cls, bg_cls) = if *is_dragging {
        ("border-primary", "bg-primary-subtle")
    } else if props.doi_count.is_some() {
        ("border-primary", "bg-primary bg-opacity-10")
    } else {
        ("border-secondary-subtle", "")
    };

    html! {
        <label
            class={classes!(
                "d-flex", "flex-column", "align-items-center", "justify-content-center",
                "border", "border-2", "rounded-3", "p-4", "w-100", "text-center", "gap-2",
                border_cls, bg_cls
            )}
            style="border-style: dashed !important; cursor: pointer; min-height: 180px;"
            ondragover={ondragover}
            ondragleave={ondragleave}
            ondrop={ondrop}
        >
            <span class="fw-semibold text-primary-emphasis">{"Your bibliography"}</span>
            if let Some(n) = props.doi_count {
                <i class="bi bi-check-circle-fill text-primary fs-3" />
                <span class="fw-medium">
                    { format!("{n} article{} found", if n == 1 { "" } else { "s" }) }
                </span>
                <small class="text-muted">{"Drop a new file to replace"}</small>
            } else {
                <i class="bi bi-upload fs-3 text-secondary" />
                <span class="fw-medium">{"Click to upload or drag & drop"}</span>
                <small class="text-muted">{".ris, .nbib, .bzd"}</small>
            }
            <input type="file" accept=".ris,.nbib,.bzd" hidden=true onchange={onchange} />
        </label>
    }
}

// ---- ExclDropZone -------------------------------------------------------

#[derive(Clone, PartialEq, Properties)]
struct ExclDropZoneProps {
    on_file: Callback<File>,
    /// Total unique DOIs accumulated across all dropped files.
    doi_count: usize,
    /// Number of files processed so far.
    file_count: usize,
}

#[function_component]
fn ExclDropZone(props: &ExclDropZoneProps) -> Html {
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
        Callback::from(move |_: DragEvent| is_dragging.set(false))
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
            if let Some(f) = file {
                on_file.emit(f);
            }
        })
    };
    let onchange = {
        let on_file = props.on_file.clone();
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            let file = input.files().and_then(|f| f.get(0)).map(File::from);
            if let Some(f) = file {
                on_file.emit(f);
            }
        })
    };

    let (border_cls, bg_cls) = if *is_dragging {
        ("border-danger", "bg-danger-subtle")
    } else if props.doi_count > 0 {
        ("border-danger", "bg-danger bg-opacity-10")
    } else {
        ("border-secondary-subtle", "")
    };

    html! {
        <label
            class={classes!(
                "d-flex", "flex-column", "align-items-center", "justify-content-center",
                "border", "border-2", "rounded-3", "p-4", "w-100", "text-center", "gap-2",
                border_cls, bg_cls
            )}
            style="border-style: dashed !important; cursor: pointer; min-height: 180px;"
            ondragover={ondragover}
            ondragleave={ondragleave}
            ondrop={ondrop}
        >
            <div class="d-flex align-items-center gap-2">
                <span class="fw-semibold text-danger-emphasis">{"Already read / exclude (optional, but recommended)"}</span>
            </div>
            if props.doi_count > 0 {
                <i class="bi bi-slash-circle text-danger fs-3" />
                <span class="fw-medium">
                    { format!(
                        "{} article{} from {} file{}",
                        props.doi_count,
                        if props.doi_count == 1 { "" } else { "s" },
                        props.file_count,
                        if props.file_count == 1 { "" } else { "s" },
                    ) }
                </span>
                <small class="text-muted">{"Drop another file to add more"}</small>
            } else {
                <i class="bi bi-slash-circle fs-3 text-secondary" />
                <span class="fw-medium">{"Click to upload or drag & drop"}</span>
                <small class="text-muted">{".ris, .nbib, .bzd \u{00b7} multiple files supported"}</small>
            }
            <input type="file" accept=".ris,.nbib,.bzd" hidden=true onchange={onchange} />
        </label>
    }
}
