use serde::Serialize;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::common::{self, SearchFor, get_value, BibliZapResultsQuery, FromSearch};
use crate::common::*;

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

/// Checks if the input string contains keywords (i.e., not all tokens are DOIs/PMIDs).
fn contains_keywords(input: &str) -> bool {
    let tokens: Vec<&str> = input.trim().split_whitespace().collect();
    if tokens.is_empty() {
        return false;
    }
    // If ANY token is not a valid DOI or PMID, treat the whole input as keywords
    tokens.iter().any(|t| !is_valid_id(t))
}

/// Query params for `/pubmed-results?q=…`
#[derive(Clone, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
pub struct PubMedResultsQuery {
    pub q: String,
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
        
        // Check if there are more than 7 IDs
        if ids.len() > 7 {
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


/// Landing page — just the centered search form.
#[function_component(BibliZapSearchPage)]
pub fn biblizap_search() -> Html {
    html! {
        <div class="form-container-centered">
            <SnowballForm />
        </div>
    }
}


/// Component for the snowball search form.
/// Allows users to input IDs or keywords, select depth, max results, and search direction.
/// On submit, navigates to `/pubmed-results?q=…` or `/biblizap-results?ids=…`.
#[function_component]
pub fn SnowballForm() -> Html {
    let navigator = use_navigator().unwrap();
    let location = use_location();

    // Pre-fill from current URL: `?ids=` on BibliZap results page, `?q=` on PubMed page.
    let prefill = location.as_ref().and_then(|l| {
        l.query::<BibliZapResultsQuery>().ok().map(|q| q.ids)
            .or_else(|| l.query::<PubMedResultsQuery>().ok().map(|q| q.q))
    }).unwrap_or_default();

    let id_list_node = use_node_ref();
    let depth_node = use_node_ref();
    let output_max_size_node = use_node_ref();
    let search_for_node = use_node_ref();

    let id_list = use_state(|| prefill);

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
        let navigator = navigator.clone();

        Callback::from(move |event: SubmitEvent| {
            event.prevent_default();

            let input_text = get_value(&id_list_node).unwrap_or_default();
            let input_trimmed = input_text.trim().to_string();

            if input_trimmed.is_empty() {
                return;
            }

            if contains_keywords(&input_trimmed) {
                // Keyword search → navigate to PubMed results page
                let _ = navigator.push_with_query_and_state(
                    &Route::PubMedResults,
                    &PubMedResultsQuery { q: input_trimmed },
                    &FromSearch,
                );
            } else {
                // DOI/PMID search → validate then navigate to BibliZap results page
                let form_content = SnowballParameters::new(
                    id_list_node.clone(),
                    depth_node.clone(),
                    output_max_size_node.clone(),
                    search_for_node.clone(),
                );

                match form_content {
                    Ok(params) => {
                        let ids_str = params.input_id_list.join(" ");
                        let _ = navigator.push_with_query_and_state(
                            &Route::BibliZapResults,
                            &BibliZapResultsQuery { ids: ids_str },
                            &FromSearch,
                        );
                    }
                    Err(_) => {}
                }
            }
        })
    };

    html! {
        <form class="container-md" onsubmit={onsubmit}>
            <div>
                <label for="idInput" class="form-label">{"Enter PMIDs, DOIs, or keywords"}</label>
                <div class="input-group input-group-lg">
                    <input type="text" class="form-control" id="idInput" placeholder={"e.g., 12345678  10.1234/example  or  breast cancer MRI"} {onchange} ref={id_list_node.clone()} value={id_list.to_string()}/>
                    <button type="submit" class="btn btn-primary">
                        <i class="bi bi-search"></i>
                        {" Search"}
                    </button>
                </div>
                <div id="idInputHelp" class="form-text">{"Enter DOIs or PMIDs to run BibliZap directly, or enter keywords to search PubMed first."}</div>
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
