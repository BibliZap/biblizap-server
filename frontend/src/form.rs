use std::cell::RefCell;
use std::rc::Rc;

use serde::Serialize;
use yew::prelude::*;

use crate::common::{self, SearchFor, get_value};

use crate::common::*;
use crate::results::article::Article;

/// Validates if a string is a valid DOI.
/// DOIs start with "10." followed by at least 4 digits, a "/", and a suffix.
fn is_valid_doi(s: &str) -> bool {
    s.starts_with("10.") 
        && s.len() > 7  // Minimum: "10.1234/x"
        && s.contains('/') 
        && s.chars().skip(3).take_while(|c| c.is_ascii_digit()).count() >= 4
}

/// Validates if a string is a valid PMID.
/// PMIDs are purely numeric identifiers.
fn is_valid_pmid(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())
}

/// Validates if a string is either a valid DOI or PMID.
fn is_valid_id(s: &str) -> bool {
    is_valid_doi(s) || is_valid_pmid(s)
}

/// Properties for the SnowballForm component.
#[derive(Clone, PartialEq, Properties)]
pub struct FormProps {
    /// Callback for when a submission error occurs.
    pub on_submit_error: Callback<common::Error>,
    /// Callback for when the search request is initiated.
    pub on_requesting_results: Callback<()>,
    /// Callback for when the search response is received.
    pub on_receiving_response: Callback<Result<Rc<RefCell<Vec<Article>>>, Error>>,
}

/// Struct representing the parameters for the snowball search API request.
#[derive(Clone, PartialEq, Debug, Default, Serialize)]
struct SnowballParameters {
    output_max_size: String,
    depth: u8,
    input_id_list: Vec<String>,
    search_for: common::SearchFor,
}

impl SnowballParameters {
    /// Creates new `SnowballParameters` from form input node references.
    fn new(
        id_list_node: NodeRef,
        depth_node: NodeRef,
        output_max_size_node: NodeRef,
        search_for_node: NodeRef,
    ) -> Result<Self, common::Error> {
        let input_string = get_value(&id_list_node)
            .ok_or(common::NodeRefMissingValue::IdList)?;
        
        let ids: Vec<String> = input_string
            .trim()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();
        
        // Check if there are more than 10 IDs
        if ids.len() > 10 {
            return Err(common::Error::TooManyIds(ids.len()));
        }
        
        // Validate each ID is either a DOI or PMID
        for id in &ids {
            if !is_valid_id(id) {
                return Err(common::Error::InvalidIdFormat(id.clone()));
            }
        }
        
        // Ensure at least one valid ID was provided
        if ids.is_empty() {
            return Err(common::Error::NoValidIds);
        }
        
        let input_id_list = ids;

        let output_max_size =
            get_value(&output_max_size_node).ok_or(common::NodeRefMissingValue::OutputMaxSize)?;

        let depth = get_value(&depth_node)
            .ok_or(common::NodeRefMissingValue::Depth)?
            .parse::<u8>()?;

        let search_for = match get_value(&search_for_node)
            .ok_or(common::NodeRefMissingValue::SearchFor)?
            .as_str()
        {
            "References" => SearchFor::References,
            "Citations" => SearchFor::Citations,
            "Both" => SearchFor::Both,
            &_ => SearchFor::Both,
        };

        Ok(SnowballParameters {
            output_max_size,
            depth,
            input_id_list,
            search_for,
        })
    }
}

/// Sends the snowball search request to the backend API.
/// Takes the form content as `SnowballParameters`.
/// Returns a `Result` containing a shared reference to a vector of `Article` or an `Error`.
async fn get_response(
    form_content: &SnowballParameters,
) -> Result<Rc<RefCell<Vec<Article>>>, Error> {
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

    let response = gloo_net::http::Request::post(api_url.as_str())
        .header("Access-Control-Allow-Origin", "*")
        .body(serde_json::to_string(&form_content)?)?
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

/// Checks the URL query parameters for a prefill `id_list_prefill`.
/// Returns the prefill string if found, otherwise `None`.
fn id_list_prefill() -> Option<String> {
    let url = gloo_utils::document().document_uri();
    let url = match url {
        Ok(href) => Ok(href),
        Err(err) => Err(Error::JsValueString(err.as_string().unwrap_or_default())),
    }
    .ok()?;

    let id_list_prefill = url::Url::parse(&url)
        .ok()?
        .query_pairs()
        .filter(|(k, _)| k.eq("id_list_prefill"))
        .map(|(_, v)| v)
        .fold(String::with_capacity(url.len()), |a, b| a + &b)
        .replace(',', " ");

    Some(id_list_prefill)
}

/// Component for the snowball search form.
/// Allows users to input IDs, select depth, max results, and search direction.
/// Handles form submission and triggers API requests.
#[function_component]
pub fn SnowballForm(props: &FormProps) -> Html {
    let id_list_node = use_node_ref();
    let depth_node = use_node_ref();
    let output_max_size_node = use_node_ref();
    let search_for_node = use_node_ref();

    let id_list = use_state(|| id_list_prefill().unwrap_or_default());

    let onchange = {
        let id_list_node = id_list_node.clone();
        let id_list = id_list.clone();
        Callback::from(move |_| {
            let input = id_list_node.cast::<web_sys::HtmlInputElement>();
            if let Some(input) = input {
                id_list.set(input.value());
            }
        })
    };

    let onsubmit: Callback<SubmitEvent> = {
        let id_list_node = id_list_node.clone();
        let depth_node = depth_node.clone();
        let output_max_size_node = output_max_size_node.clone();
        let search_for_node = search_for_node.clone();
        let on_submit_error = props.on_submit_error.clone();
        let on_receiving_response = props.on_receiving_response.clone();
        let on_requesting_results = props.on_requesting_results.clone();

        Callback::from(move |event: SubmitEvent| {
            event.prevent_default();
            on_requesting_results.emit(());

            let form_content = SnowballParameters::new(
                id_list_node.clone(),
                depth_node.clone(),
                output_max_size_node.clone(),
                search_for_node.clone(),
            );

            let form_content = match form_content {
                Ok(form_content) => form_content,
                Err(error) => {
                    on_submit_error.emit(error);
                    return;
                }
            };

            let on_receiving_response = on_receiving_response.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let response = get_response(&form_content).await;
                on_receiving_response.emit(response);
            });
        })
    };

    html! {
        <form class="container-md" onsubmit={onsubmit}>
            <div>
                <label for="idInput" class="form-label">{"Enter a list of PMIDs or DOIs (maximum 10)"}</label>
                <div class="input-group input-group-lg">
                    <input type="text" class="form-control" id="idInput" placeholder="e.g., 12345678 10.1234/example" {onchange} ref={id_list_node.clone()} value={id_list.to_string()}/>
                    <button type="submit" class="btn btn-primary">
                        <i class="bi bi-search"></i>
                        {" Search"}
                    </button>
                </div>
                <div id="idInputHelp" class="form-text">{"You can enter up to 10 identifiers separated by spaces. Only DOIs (e.g., 10.1234/example) and PMIDs (e.g., 12345678) are accepted."}</div>
            </div>
            <div class="mb-3 form-check visually-hidden">
                <div class="row">
                <div class="col">
                    <label class="form-check-label" for="depthSelect">{"Select depth"}</label>
                    <select class="form-select" aria-label="Default select example" id="depthSelect" value="2" ref={depth_node.clone()}>
                        <option value="1">{"1"}</option>
                        <option value="2" selected=true>{"2 (recommended)"}</option>
                    </select>
                    <div id="depthSelectHelp" class="form-text">{"The recommended depth value is 2"}</div>
                </div>
                <div class="col">
                    <label class="form-check-label" for="maxOutputSizeSelect">{"Number of results"}</label>
                    <select class="form-select" aria-label="Default select example" id="maxOutputSizeSelect" value="100" ref={output_max_size_node.clone()}>
                        <option value="100" selected=true>{"100"}</option>
                        <option value="500">{"500"}</option>
                        <option value="1000">{"1000"}</option>
                        <option value="All">{"All (may take longer)"}</option>
                    </select>
                </div>
                </div>
            </div>
            <div class="mb-3 form-check visually-hidden">
                <label class="form-check-label" for="searchForSelect">{"Search direction"}</label>
                <select class="form-select" aria-label="Default select example" id="searchForSelect" ref={search_for_node.clone()}>
                    <option value="Both" selected=true>{"Both"}</option>
                    <option value="Citations">{"Citations"}</option>
                    <option value="References">{"References"}</option>
                </select>
                <div id="searchForSelectHelp" class="form-text">{"For most cases, we recommend Both"}</div>
            </div>
        </form>
    }
}
