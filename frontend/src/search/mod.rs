use yew::prelude::*;
use yew_router::prelude::*;


pub mod denylist;
use denylist::*;

use crate::common::{get_value, BibliZapResultsQuery, FormPosition};
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


/// Landing page — just the centered search form.
#[function_component(BibliZapSearchPage)]
pub fn biblizap_search() -> Html {
    html! {
        <div class="form-container-centered">
            <BiblizapSearchBar position={FormPosition::Centered} value={String::new()} />
        </div>
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct AdvancedParams {
    pub depth: u8,
    pub output_max_size: OutputMaxSize,
    pub search_for: SearchFor,
    pub denylist_hash: Option<[u8; 32]>,
}

impl Default for AdvancedParams {
    fn default() -> Self {
        Self {
            depth: 2,
            output_max_size: OutputMaxSize::default(),
            search_for: SearchFor::default(),
            denylist_hash: None,
        }
    }
}

impl From<&BibliZapResultsQuery> for AdvancedParams {
    fn from(q: &BibliZapResultsQuery) -> Self {
        Self {
            depth: q.depth.unwrap_or(2),
            output_max_size: q.output_max_size.unwrap_or_default(),
            search_for: q.search_for.unwrap_or_default(),
            denylist_hash: q.denylist_hash.clone().and_then(|s| hex::decode(&s).ok().map(|v| v.try_into().unwrap_or_default())),
        }
    }
}

/// Properties for the search form component.
#[derive(Clone, PartialEq, Properties)]
pub struct SearchBarProps {
    pub position: FormPosition,
    pub value: String,
    /// Expert params pre-filled from the URL (results page). When Some and non-default,
    /// the advanced panel is auto-opened so users can see their active settings.
    #[prop_or_default]
    pub advanced: Option<AdvancedParams>,
}

fn session_get(key: &str) -> Option<String> {
    web_sys::window()?
        .session_storage()
        .ok()??
        .get_item(key)
        .ok()?
}

fn session_set(key: &str, val: &str) {
    if let Some(Ok(Some(storage))) = web_sys::window().map(|w| w.session_storage()) {
        let _ = storage.set_item(key, val);
    }
}


/// Component for the snowball search form.
/// Allows users to input IDs or keywords, select depth, max results, and search direction.
/// On submit, navigates to `/pubmed-results?q=…` or `/biblizap-results?ids=…`.
#[function_component(BiblizapSearchBar)]
pub fn biblizap_search_bar(props: &SearchBarProps) -> Html {
    let navigator = use_navigator().unwrap();
    let position = props.position;

    let id_list_node = use_node_ref();

    // ── Advanced params init (URL props > sessionStorage > hardcoded defaults) ──
    let init_advanced = props.advanced.unwrap_or_else(|| {
        let depth = session_get("bz_depth")
            .and_then(|s| s.parse().ok())
            .unwrap_or(2);
        let output_max_size = session_get("bz_output_max_size")
            .map(|s| match s.as_str() {
                "All" => OutputMaxSize::All,
                n => n.parse().ok().map(OutputMaxSize::Limit).unwrap_or_default(),
            })
            .unwrap_or_default();
        let search_for = session_get("bz_search_for")
            .map(|s| match s.as_str() {
                "Citations" => SearchFor::Citations,
                "References" => SearchFor::References,
                _ => SearchFor::Both,
            })
            .unwrap_or_default();
        let denylist_hash: Option<[u8; 32]> = session_get("bz_denylist_hash")
            .and_then(|s| hex::decode(&s).ok().map(|v| v.try_into().unwrap_or_default()));
        AdvancedParams { depth, output_max_size, search_for, denylist_hash }
    });

    // Auto-open panel if URL params are non-default.
    let init_open = props.advanced
        .map(|p| p != AdvancedParams::default())
        .unwrap_or(false)
        || session_get("bz_show_advanced").as_deref() == Some("1");

    let advanced_params = use_state(|| init_advanced);
    let show_advanced = use_state(|| init_open);

    // Keep in sync when URL-sourced props change (e.g. navigating between result pages).
    use_effect_with(props.advanced, {
        let advanced_params = advanced_params.clone();
        let show_advanced = show_advanced.clone();
        move |p| {
            if let Some(p) = p {
                advanced_params.set(*p);
                if *p != AdvancedParams::default() {
                    show_advanced.set(true);
                }
            }
            || ()
        }
    });

    let id_list = use_state(|| props.value.clone());
    use_effect_with(props.value.clone(), {
        let id_list = id_list.clone();
        move |v| {
            id_list.set(v.clone());
            || ()
        }
    });

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
        let advanced_params = advanced_params.clone();
        let navigator = navigator.clone();

        Callback::from(move |event: SubmitEvent| {
            event.prevent_default();

            let input_text = get_value(&id_list_node).unwrap_or_default();
            let input_trimmed = input_text.trim().to_string();

            if input_trimmed.is_empty() {
                return;
            }

            if contains_keywords(&input_trimmed) {
                // Keyword search → navigate to PubMed results page (expert params n/a)
                let _ = navigator.push_with_query_and_state(
                    &Route::PubMedResults,
                    PubMedResultsQuery { q: input_trimmed },
                    position.next(),
                );
            } else {
                // DOI/PMID search → validate then navigate to BibliZap results page
                let ids: Vec<String> = input_trimmed
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect();

                if ids.len() > 7 {
                    return;
                }
                if ids.iter().any(|id| !is_valid_id(id)) {
                    return;
                }
                if ids.is_empty() {
                    return;
                }

                let p = *advanced_params;
                let ids_str = ids.join(" ");

                gloo_console::log!(format!("Form position: {:#?}", position));
                gloo_console::log!(format!("Next form position: {:#?}", position.next()));

                let _ = navigator.push_with_query_and_state(
                    &Route::BibliZapResults,
                    BibliZapResultsQuery {
                        ids: ids_str,
                        depth: Some(p.depth),
                        output_max_size: Some(p.output_max_size),
                        search_for: Some(p.search_for),
                        denylist_hash: p.denylist_hash.map(hex::encode),
                    },
                    position.next(),
                );
            }
        })
    };

    html! {
        <form class="container-md" onsubmit={onsubmit}>
            <div>
                <label for="idInput" class="form-label">{"Enter PMIDs, DOIs, or keywords"}</label>
                <div class="input-group input-group-lg">
                    <input type="text" class="form-control" id="idInput"
                        placeholder={"e.g., 12345678  10.1234/example  or  breast cancer MRI"}
                        {onchange}
                        ref={id_list_node.clone()}
                        value={id_list.to_string()}
                    />
                    <SearchGear show_advanced={show_advanced.clone()} />
                    <button type="submit" class="btn btn-outline-primary">
                        <i class="bi bi-search"></i>
                        {" Search"}
                    </button>
                </div>
                <div id="idInputHelp" class="form-text">{"Enter DOIs or PMIDs to run BibliZap directly, or enter keywords to search PubMed first."}</div>
            </div>
            <SearchAdvancedPanel show_advanced={show_advanced.clone()} advanced_params={advanced_params.clone()} />
        </form>
    }
}


#[derive(Clone, PartialEq, Properties)]
struct SearchGearProps {
    show_advanced: UseStateHandle<bool>,
}
#[function_component]
fn SearchGear(props: &SearchGearProps) -> Html {
        // ── Gear button toggle ──
    let on_gear_click = {
        let show_advanced = props.show_advanced.clone();
        Callback::from(move |_: web_sys::MouseEvent| {
            let next = !*show_advanced;
            show_advanced.set(next);
            session_set("bz_show_advanced", if next { "1" } else { "0" });
        })
    };
    let gear_class = if *props.show_advanced {
        "btn btn-secondary"
    } else {
        "btn btn-outline-secondary"
    };

    html! {
        <button type="button" class={gear_class} onclick={on_gear_click} title="Advanced search options">
            <i class="bi bi-gear"></i>
        </button>
    }
}

#[derive(Clone, PartialEq, Properties)]
struct SearchAdvancedPanelProps {
    show_advanced: UseStateHandle<bool>,
    advanced_params: UseStateHandle<AdvancedParams>,
}
#[function_component]
fn SearchAdvancedPanel(props: &SearchAdvancedPanelProps) -> Html {
    let show_advanced = props.show_advanced.clone();
    let advanced_params = props.advanced_params.clone();

    let advanced_class = if *show_advanced {
        "advanced-params open"
    } else {
        "advanced-params"
    };

    let depth_str = advanced_params.depth.to_string();
    let max_str = match advanced_params.output_max_size {
        OutputMaxSize::All => "All".to_string(),
        OutputMaxSize::Limit(n) => n.to_string(),
    };
    let sf_str = match advanced_params.search_for {
        SearchFor::Both => "Both".to_string(),
        SearchFor::Citations => "Citations".to_string(),
        SearchFor::References => "References".to_string(),
    };
        // ── Select onchange callbacks ──
    let on_depth_change = {
        let advanced_params = advanced_params.clone();
        Callback::from(move |e: web_sys::Event| {
            let val = e.target_unchecked_into::<web_sys::HtmlInputElement>().value();
            let depth = val.parse().unwrap_or(2);
            session_set("bz_depth", &val);
            advanced_params.set(AdvancedParams { depth, ..*advanced_params });
        })
    };

    let on_max_change = {
        let advanced_params = advanced_params.clone();
        Callback::from(move |e: web_sys::Event| {
            let val = e.target_unchecked_into::<web_sys::HtmlInputElement>().value();
            session_set("bz_output_max_size", &val);
            let output_max_size = match val.as_str() {
                "All" => OutputMaxSize::All,
                n => n.parse().ok().map(OutputMaxSize::Limit).unwrap_or_default(),
            };
            advanced_params.set(AdvancedParams { output_max_size, ..*advanced_params });
        })
    };

    let on_search_for_change = {
        let advanced_params = advanced_params.clone();
        Callback::from(move |e: web_sys::Event| {
            let val = e.target_unchecked_into::<web_sys::HtmlInputElement>().value();
            session_set("bz_search_for", &val);
            let search_for = match val.as_str() {
                "Citations" => SearchFor::Citations,
                "References" => SearchFor::References,
                _ => SearchFor::Both,
            };
            advanced_params.set(AdvancedParams { search_for, ..*advanced_params });
        })
    };

    let on_hash_change = {
        let advanced_params = advanced_params.clone();
        Callback::from(move |hash: Option<[u8; 32]>| {
            let hash_str = hash.map(|h| hex::encode(h)).unwrap_or_default();
            session_set("bz_denylist_hash", &hash_str);
            advanced_params.set(AdvancedParams { denylist_hash: hash, ..*advanced_params });
        })
    };

    html! {
    <div class={advanced_class}>
        <div>
            <div class="row g-3 pt-2">
                <div class="col-sm">
                    <label class="form-label" for="depthSelect">{"Depth"}</label>
                    <select class="form-select form-select-sm" id="depthSelect"
                        value={depth_str}
                        onchange={on_depth_change}
                    >
                        <option value="1" selected={advanced_params.depth == 1}>{"1"}</option>
                        <option value="2" selected={advanced_params.depth == 2}>{"2 (recommended)"}</option>
                    </select>
                    <div class="form-text">{"Recommended: 2"}</div>
                </div>
                <div class="col-sm">
                    <label class="form-label" for="maxOutputSizeSelect">{"Number of results"}</label>
                    <select class="form-select form-select-sm" id="maxOutputSizeSelect"
                        value={max_str}
                        onchange={on_max_change}
                    >
                        <option value="100"   selected={advanced_params.output_max_size == OutputMaxSize::Limit(100)}>{"100"}</option>
                        <option value="500"   selected={advanced_params.output_max_size == OutputMaxSize::Limit(500)}>{"500"}</option>
                        <option value="1000"  selected={advanced_params.output_max_size == OutputMaxSize::Limit(1000)}>{"1000"}</option>
                        <option value="All"   selected={advanced_params.output_max_size == OutputMaxSize::All}>{"All (may take longer)"}</option>
                    </select>
                </div>
                <div class="col-sm">
                    <label class="form-label" for="searchForSelect">{"Search direction"}</label>
                    <select class="form-select form-select-sm" id="searchForSelect"
                        value={sf_str}
                        onchange={on_search_for_change}
                    >
                        <option value="Both"       selected={advanced_params.search_for == SearchFor::Both}>{"Both (recommended)"}</option>
                        <option value="Citations"  selected={advanced_params.search_for == SearchFor::Citations}>{"Citations"}</option>
                        <option value="References" selected={advanced_params.search_for == SearchFor::References}>{"References"}</option>
                    </select>
                </div>
                <Denylist on_hash_change={on_hash_change} initial_hash={advanced_params.denylist_hash} />
            </div>
        </div>
    </div>
    }
}