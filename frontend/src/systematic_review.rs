use std::{collections::HashSet, rc::Rc};

use gloo_file::{callbacks::read_as_text, futures, File, FileList};
use wasm_bindgen_futures::spawn_local;
use web_sys::{DragEvent, HtmlInputElement};
use yew::prelude::*;
use yew_router::prelude::*;

use crate::common::{Route, SeedSelectionQuery, MAX_SEEDS};
use crate::search::denylist::{extract_dois, upload_denylist_to_backend};

#[derive(Clone)]
struct ExclusionFile {
    name: String,
    dois: Vec<String>,
}

#[derive(Default)]
struct ExclusionFiles(Vec<ExclusionFile>);

enum ExclusionAction {
    Add(Vec<ExclusionFile>),
    Remove(usize),
}

impl Reducible for ExclusionFiles {
    type Action = ExclusionAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        let mut files = self.0.clone();
        match action {
            ExclusionAction::Add(new_files) => files.extend(new_files),
            ExclusionAction::Remove(index) => {
                if index < files.len() {
                    files.remove(index);
                }
            }
        }
        Rc::new(Self(files))
    }
}

fn unique_exclusion_doi_count(files: &[ExclusionFile]) -> usize {
    files
        .iter()
        .flat_map(|file| file.dois.iter().map(String::as_str))
        .collect::<HashSet<_>>()
        .len()
}

#[function_component]
pub fn SystematicReviewPage() -> Html {
    let navigator = use_navigator().unwrap();
    let uploading = use_state(|| false);
    let error = use_state(|| Option::<String>::None);
    let bib_dois = use_state(|| Option::<Vec<String>>::None);
    let excl_files = use_reducer(ExclusionFiles::default);
    let excl_reading = use_mut_ref(|| false);
    let is_reading_excl = use_state(|| false);
    let bib_reader = use_mut_ref(|| None);

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

    let on_remove_bib = {
        let bib_dois = bib_dois.clone();
        let bib_reader = bib_reader.clone();
        Callback::from(move |_: MouseEvent| {
            // Dropping the reader cancels an in-progress file read as well.
            *bib_reader.borrow_mut() = None;
            bib_dois.set(None);
        })
    };

    let on_excl_files = {
        let excl_files = excl_files.clone();
        let excl_reading = excl_reading.clone();
        let is_reading_excl = is_reading_excl.clone();
        let error = error.clone();
        Callback::from(move |files: Vec<File>| {
            if files.is_empty() || *excl_reading.borrow() {
                return;
            }
            *excl_reading.borrow_mut() = true;
            is_reading_excl.set(true);
            error.set(None);
            let excl_files = excl_files.clone();
            let excl_reading = excl_reading.clone();
            let is_reading_excl = is_reading_excl.clone();
            let error = error.clone();
            spawn_local(async move {
                let mut loaded = Vec::new();
                let mut errors = Vec::new();
                for file in files {
                    let name = file.name();
                    match futures::read_as_text(&file).await {
                        Ok(content) => match extract_dois(&content) {
                            Some(mut dois) if !dois.is_empty() => {
                                dois.sort_unstable();
                                dois.dedup();
                                loaded.push(ExclusionFile { name, dois });
                            }
                            _ => errors.push(format!("{name}: no DOIs found.")),
                        },
                        Err(_) => errors.push(format!("{name}: failed to read file.")),
                    }
                }
                if !loaded.is_empty() {
                    excl_files.dispatch(ExclusionAction::Add(loaded));
                }
                if !errors.is_empty() {
                    error.set(Some(errors.join(" ")));
                }
                *excl_reading.borrow_mut() = false;
                is_reading_excl.set(false);
            });
        })
    };

    let on_remove_excl = {
        let excl_files = excl_files.clone();
        Callback::from(move |index: usize| excl_files.dispatch(ExclusionAction::Remove(index)))
    };

    let on_continue = {
        let bib_dois = bib_dois.clone();
        let excl_files = excl_files.clone();
        let uploading = uploading.clone();
        let error = error.clone();
        let navigator = navigator.clone();
        Callback::from(move |_: MouseEvent| {
            let Some(bib) = (*bib_dois).clone() else {
                return;
            };
            let excl = excl_files.0.clone();
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
                let mut excl_hashes = Vec::new();
                for file in excl {
                    match upload_denylist_to_backend(file.dois).await {
                        Ok(hash) => excl_hashes.push(hex::encode(hash)),
                        Err(e) => {
                            error.set(Some(format!(
                                "Exclusion upload failed for {}: {e}",
                                file.name
                            )));
                            uploading_c.set(false);
                            return;
                        }
                    }
                }
                let _ = navigator.push_with_query(
                    &Route::SeedSelection,
                    &SeedSelectionQuery {
                        bibliography: hex::encode(bib_hash),
                        denylists: (!excl_hashes.is_empty()).then(|| excl_hashes.join(" ")),
                    },
                );
            });
        })
    };

    let bib_doi_count = (*bib_dois).as_ref().map(|v| v.len());
    let excl_doi_count = unique_exclusion_doi_count(&excl_files.0);
    let excl_fc = excl_files.0.len();
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
                    if bib_doi_count.is_some() {
                        <button type="button" class="btn btn-outline-secondary btn-sm mt-2" onclick={on_remove_bib} disabled={is_uploading}>
                            {"Remove bibliography"}
                        </button>
                    }
                </div>
                <div class="col-md-6">
                    <ExclDropZone on_files={on_excl_files} doi_count={excl_doi_count} file_count={excl_fc} is_reading={*is_reading_excl} />
                    if !excl_files.0.is_empty() {
                        <ul class="list-group mt-2" aria-label="Uploaded exclusion files">
                            { excl_files.0.iter().enumerate().map(|(index, file)| {
                                let on_remove_excl = on_remove_excl.clone();
                                let on_remove = Callback::from(move |_: MouseEvent| on_remove_excl.emit(index));
                                html! {
                                    <li class="list-group-item d-flex justify-content-between align-items-center gap-2">
                                        <span class="text-truncate" title={file.name.clone()}>{ format!("{} ({} articles)", file.name, file.dois.len()) }</span>
                                        <button type="button" class="btn btn-outline-danger btn-sm" aria-label={format!("Remove {}", file.name)} onclick={on_remove} disabled={is_uploading}>
                                            <i class="bi bi-x-lg" />
                                        </button>
                                    </li>
                                }
                            }).collect::<Html>() }
                        </ul>
                    }
                </div>
            </div>
            <div class="d-flex align-items-center gap-3">
                <button
                    class="btn btn-primary"
                    onclick={on_continue}
                    disabled={is_uploading || *is_reading_excl || bib_doi_count.is_none()}
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
            <small class="text-muted">
                {format!("You can select up to {MAX_SEEDS} seed articles. ")}
                <a href="mailto:contact@biblizap.org">{"Contact us"}</a>
                {" to request a higher limit."}
            </small>
            <input type="file" accept=".ris,.nbib,.bzd" hidden=true onchange={onchange} />
        </label>
    }
}

// ---- ExclDropZone -------------------------------------------------------

#[derive(Clone, PartialEq, Properties)]
struct ExclDropZoneProps {
    on_files: Callback<Vec<File>>,
    /// Total unique DOIs accumulated across all dropped files.
    doi_count: usize,
    /// Number of files currently included.
    file_count: usize,
    is_reading: bool,
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
        let on_files = props.on_files.clone();
        let is_reading = props.is_reading;
        Callback::from(move |e: DragEvent| {
            e.prevent_default();
            is_dragging.set(false);
            if is_reading {
                return;
            }
            let files = e
                .data_transfer()
                .and_then(|dt| dt.files())
                .map(|files| FileList::from(files).iter().cloned().collect::<Vec<_>>())
                .unwrap_or_default();
            if !files.is_empty() {
                on_files.emit(files);
            }
        })
    };
    let onchange = {
        let on_files = props.on_files.clone();
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            let files = input
                .files()
                .map(|files| FileList::from(files).iter().cloned().collect::<Vec<_>>())
                .unwrap_or_default();
            if !files.is_empty() {
                on_files.emit(files);
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
            if props.is_reading {
                <span class="text-muted" role="status">
                    <span class="spinner-border spinner-border-sm me-2" />
                    {"Reading files…"}
                </span>
            }
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
                <small class="text-muted">{"Select or drop more files to add them"}</small>
            } else {
                <i class="bi bi-slash-circle fs-3 text-secondary" />
                <span class="fw-medium">{"Click to upload or drag & drop"}</span>
                <small class="text-muted">{".ris, .nbib, .bzd \u{00b7} multiple files supported"}</small>
            }
            <small class="text-muted">
                {"There is no limit on already read / excluded articles. "}
            </small>
            <input type="file" accept=".ris,.nbib,.bzd" multiple=true disabled={props.is_reading} hidden=true onchange={onchange} />
        </label>
    }
}
