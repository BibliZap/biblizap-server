use std::collections::HashMap;

use gloo_file::{futures::read_as_text, FileList};
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;
use yew::prelude::*;

use crate::upload_limit::CORPUS_UPLOAD_LIMIT_MIB;

#[derive(Debug, thiserror::Error)]
pub enum DenylistError {
    #[error("Network error: {0}")]
    NetworkError(#[from] gloo_net::Error),
    #[error("Backend status code: {0}")]
    BackendError(u16),
    #[error("Upload too large (BibliZap limit: {limit_mib} MiB of extracted identifiers; a proxy may allow less)")]
    UploadTooLarge { limit_mib: usize },
    #[error("Invalid hash format")]
    InvalidHashFormat,
}

pub fn decode_denylist_hash(hash_str: &str) -> Result<[u8; 32], DenylistError> {
    if let Ok(bytes) = hex::decode(hash_str) {
        if bytes.len() == 32 {
            let mut hash = [0u8; 32];
            hash.copy_from_slice(&bytes);
            return Ok(hash);
        }
    }
    Err(DenylistError::InvalidHashFormat)
}

pub async fn upload_denylist_to_backend(dois: Vec<String>) -> Result<[u8; 32], DenylistError> {
    let body = dois.join("\n");
    if body.len() > CORPUS_UPLOAD_LIMIT_MIB * 1024 * 1024 {
        return Err(DenylistError::UploadTooLarge {
            limit_mib: CORPUS_UPLOAD_LIMIT_MIB,
        });
    }
    let response = gloo_net::http::Request::post("/api/corpus/upload")
        .header("Content-Type", "text/plain")
        .body(body)?
        .send()
        .await?;
    if response.status() == 413 {
        return Err(DenylistError::UploadTooLarge {
            limit_mib: CORPUS_UPLOAD_LIMIT_MIB,
        });
    }
    if !response.ok() {
        return Err(DenylistError::BackendError(response.status()));
    }
    let hex_str = response.text().await?;
    let hash = decode_denylist_hash(&hex_str)?;
    Ok(hash)
}

pub async fn download_denylist(hash: [u8; 32]) -> Result<Vec<String>, DenylistError> {
    let hash_hex = hex::encode(hash);
    let response = gloo_net::http::Request::get(&format!("/api/corpus/download/{}", hash_hex))
        .send()
        .await?;
    if response.ok() {
        let text = response.text().await?;
        Ok(text
            .lines()
            .filter(|l| !l.is_empty())
            .map(|s| s.to_string())
            .collect())
    } else {
        Err(DenylistError::BackendError(response.status()))
    }
}

#[derive(Clone, PartialEq, Properties)]
pub struct DenylistProps {
    /// Current list of corpus hashes, one per chip.
    pub hashes: Vec<[u8; 32]>,
    /// Called once with all successfully uploaded files; parent appends their hashes.
    pub on_add: Callback<Vec<[u8; 32]>>,
    /// Called with the chip index when the user clicks ✕.
    pub on_remove: Callback<usize>,
}

#[function_component]
pub fn Denylist(props: &DenylistProps) -> Html {
    // Cache hash → article count; each hash is fetched at most once.
    let count_cache: UseStateHandle<HashMap<[u8; 32], usize>> = use_state(HashMap::new);
    let uploading = use_state(|| false);
    let upload_error = use_state(|| None::<String>);

    // For any hash not yet in the cache, download it and store its count.
    use_effect_with(props.hashes.clone(), {
        let count_cache = count_cache.clone();
        move |hashes| {
            let to_fetch: Vec<[u8; 32]> = hashes
                .iter()
                .filter(|h| !(*count_cache).contains_key(*h))
                .copied()
                .collect();
            if !to_fetch.is_empty() {
                let count_cache = count_cache.clone();
                spawn_local(async move {
                    for hash in to_fetch {
                        let count = download_denylist(hash).await.map(|d| d.len()).unwrap_or(0);
                        let mut new_cache = (*count_cache).clone();
                        new_cache.insert(hash, count);
                        count_cache.set(new_cache);
                    }
                });
            }
            || ()
        }
    });

    let on_file_change = {
        let uploading = uploading.clone();
        let on_add = props.on_add.clone();
        let count_cache = count_cache.clone();
        let upload_error = upload_error.clone();
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            let Some(files) = input.files() else {
                return;
            };
            let files = FileList::from(files);
            if files.is_empty() {
                return;
            }
            upload_error.set(None);
            uploading.set(true);
            let uploading = uploading.clone();
            let on_add = on_add.clone();
            let count_cache = count_cache.clone();
            let upload_error = upload_error.clone();
            spawn_local(async move {
                let mut uploaded = Vec::new();
                let mut errors = Vec::new();
                for file in files.iter() {
                    let filename = file.name();
                    let content = match read_as_text(file).await {
                        Ok(content) => content,
                        Err(_) => {
                            errors.push(format!("{filename}: failed to read file."));
                            continue;
                        }
                    };
                    let Some(dois) = extract_dois(&content).filter(|dois| !dois.is_empty()) else {
                        errors.push(format!("{filename}: no DOIs found."));
                        continue;
                    };
                    let count = dois.len();
                    match upload_denylist_to_backend(dois).await {
                        Ok(hash) => uploaded.push((hash, count)),
                        Err(error) => errors.push(format!("{filename}: {error}")),
                    }
                }
                if !uploaded.is_empty() {
                    // Update the parent once so a multi-file selection cannot lose
                    // hashes to stale state between individual on_add callbacks.
                    let mut new_cache = (*count_cache).clone();
                    let hashes = uploaded
                        .into_iter()
                        .map(|(hash, count)| {
                            new_cache.insert(hash, count);
                            hash
                        })
                        .collect();
                    count_cache.set(new_cache);
                    on_add.emit(hashes);
                }
                if !errors.is_empty() {
                    upload_error.set(Some(errors.join(" ")));
                }
                uploading.set(false);
            });
        })
    };

    html! {
        <div class="d-flex flex-wrap gap-2 align-items-center">
            { props.hashes.iter().enumerate().map(|(i, hash)| {
                let count = (*count_cache).get(hash).copied();
                let on_remove = {
                    let on_remove = props.on_remove.clone();
                    Callback::from(move |_: ()| on_remove.emit(i))
                };
                html! { <DenylistChip {count} {on_remove} /> }
            }).collect::<Html>() }
            if !*uploading {
                <DenylistUploadButton on_file_change={on_file_change} />
            } else {
                <DenylistLoading />
            }
            if let Some(error) = &*upload_error {
                <span class="text-danger" role="alert">{error}</span>
            }
        </div>
    }
}

#[derive(Clone, PartialEq, Properties)]
struct DenylistChipProps {
    /// None = count still loading.
    count: Option<usize>,
    on_remove: Callback<()>,
}

#[function_component]
fn DenylistChip(props: &DenylistChipProps) -> Html {
    let on_close = {
        let on_remove = props.on_remove.clone();
        Callback::from(move |_: MouseEvent| on_remove.emit(()))
    };
    html! {
        <div class="denylist btn btn-success btn-sm mb-0 d-flex align-items-center gap-2">
            if let Some(n) = props.count {
                <span>{ format!("{n} articles excluded") }</span>
            } else {
                <div class="spinner-border spinner-border-sm" role="status" />
                <span>{"Loading..."}</span>
            }
            <button class="btn-close btn-close-white btn-sm" aria-label="Remove" onclick={on_close} />
        </div>
    }
}

#[derive(Clone, PartialEq, Properties)]
struct DenylistUploadButtonProps {
    on_file_change: Callback<Event>,
}

#[function_component]
fn DenylistUploadButton(
    DenylistUploadButtonProps { on_file_change }: &DenylistUploadButtonProps,
) -> Html {
    html! {
        <label class="btn btn-outline-secondary btn-sm mb-0">
            <i class="bi bi-upload me-1" />
            {"Add articles to exclude from results"}
            <input type="file" accept=".ris,.nbib,.bzd" multiple=true hidden=true onchange={on_file_change} />
        </label>
    }
}

#[function_component]
fn DenylistLoading() -> Html {
    html! {
        <div class="denylist denylist-loading btn btn-outline-secondary btn-sm mb-0">
            <div class="spinner-border spinner-border-sm" />
            <span>{"Uploading..."}</span>
        </div>
    }
}

#[derive(Debug, Clone, Copy)]
enum FileType {
    Ris,
    Nbib,
    FlatDoiList,
}

impl FileType {
    // This inference doesn't need to be perfect
    fn infer_from_raw_string(raw_string: &str) -> Option<Self> {
        if raw_string.is_empty() {
            return None;
        }
        // Only test on the first 15 lines
        let sample_lines: Vec<&str> = raw_string
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .take(15)
            .collect();

        if sample_lines.iter().any(|l| l.starts_with("TY  - ")) {
            return Some(Self::Ris);
        }

        if sample_lines.iter().any(|l| l.starts_with("PMID- ")) {
            return Some(Self::Nbib);
        }

        let is_doi_list = sample_lines
            .iter()
            .all(|l| l.contains("10.") && l.contains('/'));

        if is_doi_list {
            return Some(Self::FlatDoiList);
        }

        None
    }
}

pub fn extract_dois(raw_string: &str) -> Option<Vec<String>> {
    match FileType::infer_from_raw_string(raw_string)? {
        FileType::Ris => Some(extract_from_ris(raw_string)),
        FileType::Nbib => Some(extract_from_nbib(raw_string)),
        FileType::FlatDoiList => Some(extract_from_flat_doi_list(raw_string)),
    }
}

fn extract_from_ris(ris_content: &str) -> Vec<String> {
    fn extract_from_ris_line(line: &str) -> Option<String> {
        if line.starts_with("DO  - ") {
            Some(line[6..].trim().to_string())
        } else {
            None
        }
    }

    ris_content
        .lines()
        .filter_map(|line| extract_from_ris_line(line))
        .collect()
}

fn extract_from_nbib(nbib_content: &str) -> Vec<String> {
    let mut dois = Vec::new();
    let mut found_doi_for_current_record = false;

    for line in nbib_content.lines() {
        // Reset our tracker when we hit a new record (indicated by PMID)
        if line.starts_with("PMID- ") {
            found_doi_for_current_record = false;
            continue;
        }

        // If we haven't found a DOI for this record yet, look for one
        if !found_doi_for_current_record && line.trim_end().ends_with("[doi]") {
            if let Some((_, value)) = line.split_once("- ") {
                let clean_doi = value.trim_end_matches("[doi]").trim();
                dois.push(clean_doi.to_string());

                // Lock it down so we ignore any subsequent AID/LID duplicates in this same record
                found_doi_for_current_record = true;
            }
        }
    }

    dois
}

fn extract_from_flat_doi_list(flat_doi_list: &str) -> Vec<String> {
    flat_doi_list.lines().map(|l| l.to_string()).collect()
}
