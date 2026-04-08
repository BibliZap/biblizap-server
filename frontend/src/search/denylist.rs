use gloo_file::{callbacks::read_as_text, File, FileReadError};
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;
use yew::prelude::*;

#[derive(Debug, thiserror::Error)]
pub enum DenylistError {
    #[error("Network error: {0}")]
    NetworkError(#[from] gloo_net::Error),
    #[error("Backend status code: {0}")]
    BackendError(u16),
    #[error("Invalid hash format")]
    InvalidHashFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct UploadState {
    hash: [u8; 32],
    count: usize,
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
    let response = gloo_net::http::Request::post("/api/denylist/upload")
        .header("Content-Type", "text/plain")
        .body(body)?
        .send()
        .await?;
    let hex_str = response.text().await?;
    let hash = decode_denylist_hash(&hex_str)?;
    Ok(hash)
}

pub async fn download_denylist(hash: [u8; 32]) -> Result<Vec<String>, DenylistError> {
    let hash_hex = hex::encode(hash);
    let response = gloo_net::http::Request::get(&format!("/api/denylist/download/{}", hash_hex))
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

#[derive(Debug, Clone, PartialEq, Eq)]
enum DenylistState {
    None,
    Loading,
    FrontendLoaded(Vec<String>),
    BackendUploaded(UploadState),
}

#[derive(Clone, PartialEq, Properties)]
pub struct DenylistProps {
    pub on_hash_change: Callback<Option<[u8; 32]>>,
    #[prop_or_default]
    pub initial_hash: Option<[u8; 32]>,
}

fn update_denylist_state_from_hash(
    hash: Option<[u8; 32]>,
    denylist_state: UseStateHandle<DenylistState>,
) {
    if let Some(hash) = hash {
        denylist_state.set(DenylistState::Loading);
        spawn_local(async move {
            let result = download_denylist(hash).await;
            match result {
                Ok(dois) => denylist_state.set(DenylistState::BackendUploaded(UploadState {
                    hash,
                    count: dois.len(),
                })),
                Err(_) => denylist_state.set(DenylistState::None),
            }
        });
    } else {
        denylist_state.set(DenylistState::None);
    }
}

#[function_component]
pub fn Denylist(props: &DenylistProps) -> Html {
    let denylist_state = use_state(|| DenylistState::None);
    use_effect_with(props.initial_hash, {
        let denylist_state = denylist_state.clone();
        move |hash| update_denylist_state_from_hash(*hash, denylist_state.clone())
    });

    let reader_task = use_mut_ref(|| None);

    let on_file_change = {
        let denylist_state = denylist_state.clone();
        let reader_task = reader_task.clone();
        let on_hash_change = props.on_hash_change.clone();
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            let file = File::from(input.files().unwrap().get(0).unwrap());
            denylist_state.set(DenylistState::Loading);
            let file_content = denylist_state.clone();
            let on_hash_change = on_hash_change.clone();
            let task = read_as_text(&file, move |result| {
                apply_file_read(result, file_content.clone(), on_hash_change.clone())
            });
            *reader_task.borrow_mut() = Some(task);
        })
    };

    let on_file_remove = {
        let denylist_state = denylist_state.clone();
        let on_hash_change = props.on_hash_change.clone();
        Callback::from(move |_: ()| {
            denylist_state.set(DenylistState::None);
            *reader_task.borrow_mut() = None;
            on_hash_change.emit(None);
        })
    };

    html! {
        <div>
            { match (*denylist_state).clone() {
                DenylistState::None => html! { <DenylistUploadButton on_file_change={on_file_change} /> },
                DenylistState::Loading => html! { <DenylistLoading /> },
                DenylistState::FrontendLoaded(_) => html! { <DenylistLoading /> },
                DenylistState::BackendUploaded(upload_state) => html! { <DenylistDisplay upload_state={upload_state.clone()} on_file_remove={on_file_remove} /> },
            }}
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
            {"Upload deny list"}
            <input type="file" accept=".ris" hidden=true onchange={on_file_change} />
        </label>
    }
}

#[function_component]
fn DenylistLoading() -> Html {
    html! {
        <div class="denylist denylist-loading btn btn-outline-secondary btn-sm mb-0">
            <div class="spinner-border spinner-border-sm" />
            <span>{"Loading deny list..."}</span>
        </div>
    }
}

#[derive(Clone, PartialEq, Properties)]
struct DenylistDisplayProps {
    upload_state: UploadState,
    on_file_remove: Callback<()>,
}

#[function_component]
fn DenylistDisplay(props: &DenylistDisplayProps) -> Html {
    let on_close_click = {
        let on_file_remove = props.on_file_remove.clone();
        Callback::from(move |_: MouseEvent| on_file_remove.emit(()))
    };
    let hash_hex = hex::encode(props.upload_state.hash);
    let download_href = format!("/api/denylist/download/{}", hash_hex);
    let download_filename = format!("denylist_{}.txt", &hash_hex[..8]);
    html! {
        <div class="denylist btn btn-success btn-sm mb-0 d-flex align-items-center gap-2">
            <a class="text-white text-decoration-none flex-grow-1" href={download_href} download={download_filename}>
                { format!("{} articles in deny list", props.upload_state.count) }
            </a>
            <button class="btn-close btn-close-white btn-sm" aria-label="Remove deny list" onclick={on_close_click} />
        </div>
    }
}

fn apply_file_read(
    result: Result<String, FileReadError>,
    file_content: UseStateHandle<DenylistState>,
    on_hash_change: Callback<Option<[u8; 32]>>,
) {
    match result {
        Ok(content) => {
            let dois = extract_dois_from_ris(&content);
            file_content.set(DenylistState::FrontendLoaded(dois.clone()));
            spawn_local(async move {
                let hash = upload_denylist_to_backend(dois.clone()).await.unwrap();
                on_hash_change.emit(Some(hash));
                file_content.set(DenylistState::BackendUploaded(UploadState {
                    hash,
                    count: dois.len(),
                }));
            });
        }
        Err(_) => file_content.set(DenylistState::None),
    }
}

fn extract_dois_from_ris(content: &str) -> Vec<String> {
    content
        .lines()
        .filter_map(|line| {
            if line.starts_with("DO  - ") {
                Some(line[6..].trim().to_string())
            } else {
                None
            }
        })
        .collect()
}
