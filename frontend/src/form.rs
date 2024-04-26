use std::cell::RefCell;
use std::rc::Rc;

use serde::Serialize;
use yew::prelude::*;

use crate::common::{self, SearchFor, get_value};

use crate::table::article::Article;
use crate::common::*;

#[derive(Clone, PartialEq, Properties)]
pub struct FormProps {
    pub on_submit_error: Callback<common::Error>,
    pub on_requesting_results: Callback<()>,
    pub on_receiving_response: Callback<Result<Rc<RefCell<Vec<Article>>>, Error>>,
}

#[derive(Clone, PartialEq, Properties, Debug, Default, Serialize)]
struct SnowballParameters {
    output_max_size: usize,
    depth: u8,
    input_id_list: Vec<String>,
    search_for: common::SearchFor
}

impl SnowballParameters {
    fn new(id_list_node: NodeRef,
            depth_node: NodeRef,
            output_max_size_node: NodeRef,
            search_for_node: NodeRef) -> Result<Self, common::Error> {

        let input_id_list = get_value(&id_list_node)
            .ok_or(common::NodeRefMissingValue::IdList)?
            .trim()
            .split(' ')
            .map(str::to_string)
            .collect::<Vec<String>>();
        
        let output_max_size = get_value(&output_max_size_node)
            .ok_or(common::NodeRefMissingValue::OutputMaxSize)?
            .parse::<usize>()?;

        let depth = get_value(&depth_node)
            .ok_or(common::NodeRefMissingValue::Depth)?
            .parse::<u8>()?;
        
        let search_for = match get_value(&search_for_node).ok_or(common::NodeRefMissingValue::SearchFor)?.as_str() {
                "References" => SearchFor::References,
                "Citations" => SearchFor::Citations,
                "Both" => SearchFor::Both,
                &_ => SearchFor::Both
            };

        Ok(SnowballParameters {
            output_max_size,
            depth,
            input_id_list,
            search_for
        })
    }
}

async fn get_response(form_content: &SnowballParameters) -> Result<Rc<RefCell<Vec<Article>>>, Error> {
    use gloo_utils::document;
    let url = document().document_uri();
    let url = match url {
        Ok(href) => Ok(href),
        Err(err) => Err(Error::JsValueString(err.as_string().unwrap_or_default()))
    }?.replace('#', "");

    let mut api_url = url::Url::parse(&url)?;
    api_url.set_fragment("".into());
    api_url.set_query("".into());
    api_url.set_path("api");

    let response = gloo_net::http::Request::post(api_url.as_str())
        .header("Access-Control-Allow-Origin", "*")
        .body(serde_json::to_string(&form_content)?)?
        .send()
        .await?
        .text()
        .await?;

    let value = serde_json::from_str::<serde_json::Value>(&response)?;
    let mut articles = serde_json::from_value::<Vec<Article>>(value)?;

    articles.sort_by_key(|article| std::cmp::Reverse(article.score.unwrap_or_default()));
    
    Ok(Rc::new(RefCell::new(articles)))
}

fn id_list_prefill() -> Option<String> {
    let url = gloo_utils::document().document_uri();
    let url = match url {
        Ok(href) => Ok(href),
        Err(err) => Err(Error::JsValueString(err.as_string().unwrap_or_default()))
    }.ok()?;

    let id_list_prefill = url::Url::parse(&url).ok()?
        .query_pairs()
        .filter(|(k, _)| k.eq("id_list_prefill"))
        .map(|(_,v)| v)
        .fold(String::with_capacity(url.len()), |a, b| a+&b)
        .replace(',', " ");

    Some(id_list_prefill)
}

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
            
            let form_content = SnowballParameters::new(id_list_node.clone(),
                    depth_node.clone(),
                    output_max_size_node.clone(),
                    search_for_node.clone());

            let form_content = match form_content {
                Ok(form_content) => form_content,
                Err(error) => {
                    on_submit_error.emit(error);
                    return
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
        <form class="container-md" onsubmit={onsubmit} style={"margin-bottom: 50px;"}>
            <div class="mb-3 form-check">
                <label for="idInput" class="form-label">{"Enter a list of PMIDs, DOIs or Lens IDs"}</label>
                <input type="text" class="form-control" id="idInput" {onchange} ref={id_list_node.clone()} value={id_list.to_string()}/>
                <div id="idInputHelp" class="form-text">{"You can enter multiple references separated by spaces."}</div>
            </div>
            <div class="mb-3 form-check">
                <div class="row">
                <div class="col">
                    <label class="form-check-label" for="depthSelect">{"Select depth"}</label>
                    <select class="form-select" aria-label="Default select example" id="depthSelect" value="2" ref={depth_node.clone()}>
                        <option value="1">{"1"}</option>
                        <option value="2" selected=true>{"2"}</option>
                    </select>
                    <div id="depthSelectHelp" class="form-text">{"The recommended depth value is 2"}</div>
                </div>
                <div class="col">
                    <label class="form-check-label" for="maxOutputSizeSelect">{"Number of results"}</label>
                    <select class="form-select" aria-label="Default select example" id="maxOutputSizeSelect" value="100" ref={output_max_size_node.clone()}>
                        <option value="100" selected=true>{"100"}</option>
                        <option value="500">{"500"}</option>
                        <option value="1000">{"1000"}</option>
                    </select>
                </div>
                </div>
            </div>
            <div class="mb-3 form-check">
                <label class="form-check-label" for="searchForSelect">{"Search direction"}</label>
                <select class="form-select" aria-label="Default select example" id="searchForSelect" ref={search_for_node.clone()}>
                    <option value="Both" selected=true>{"Both"}</option>
                    <option value="Citations">{"Citations"}</option>
                    <option value="References">{"References"}</option>
                </select>
                <div id="searchForSelectHelp" class="form-text">{"For most cases, we recommend Both"}</div>
            </div>
            <div class="text-center">
                <button type="submit" class="btn btn-outline-secondary btn-lg">{"Search for related articles"}</button>
            </div>
        </form>
    }
}
