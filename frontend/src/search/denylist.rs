use gloo_file::{callbacks::read_as_text, File, FileReadError};
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;
use yew::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct UploadState {
    hash: String,
    count: usize,
}

pub async fn upload_denylist_to_backend(dois: Vec<String>) -> Result<String, gloo_net::Error> {
    let body = dois.join("\n");
    let response = gloo_net::http::Request::post("/api/denylist/upload")
        .header("Content-Type", "text/plain")
        .body(body)?
        .send()
        .await?;
    Ok(response.text().await?.trim().to_string())
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum DenylistState {
    None,
    Loading,
    FrontendLoaded(Vec<String>),
    BackendUploaded(UploadState),
}

#[function_component]
pub fn Denylist() -> Html {
    let file_content = use_state(|| DenylistState::None);
    let reader_task = use_mut_ref(|| None);

    let on_file_change = {
        let file_content = file_content.clone();
        let reader_task = reader_task.clone();
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            let file = File::from(input.files().unwrap().get(0).unwrap());
            file_content.set(DenylistState::Loading);
            let file_content = file_content.clone();
            let task = read_as_text(&file, move |result| {
                apply_file_read(result, file_content.clone())
            });
            *reader_task.borrow_mut() = Some(task);
        })
    };

    let on_file_remove = {
        let file_content = file_content.clone();
        Callback::from(move |_: ()| {
            file_content.set(DenylistState::None);
            *reader_task.borrow_mut() = None;
        })
    };

    html! {
        <div>
            { match (*file_content).clone() {
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
    let download_href = format!("/api/denylist/download/{}", props.upload_state.hash);
    let download_filename = format!("denylist_{}.txt", props.upload_state.hash);
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
) {
    match result {
        Ok(content) => {
            let dois = extract_dois_from_ris(&content);
            file_content.set(DenylistState::FrontendLoaded(dois.clone()));
            spawn_local(async move {
                let hash = upload_denylist_to_backend(dois.clone()).await.unwrap();
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
