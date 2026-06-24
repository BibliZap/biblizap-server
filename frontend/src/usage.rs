use std::collections::HashMap;

use yew::prelude::*;

use crate::results::{ErrorMessage, Spinner};

#[derive(Debug, serde::Serialize, Clone, Copy, PartialEq, Eq, Hash, Default)]
enum SamplingTime {
    Day,
    #[default]
    Week,
    Month,
    Year,
    All,
}

enum PageState {
    Loading,
    Loaded(HashMap<i64, i64>),
    Error(String),
}

async fn fetch_usage_data(
    sampling_time: SamplingTime,
) -> Result<HashMap<i64, i64>, Box<dyn std::error::Error>> {
    let body = serde_json::to_string(&sampling_time)?;
    let response = gloo_net::http::Request::post("/api/usage_info/")
        .header("Content-Type", "text/plain")
        .body(body)?
        .send()
        .await?;
    let data: HashMap<i64, i64> = serde_json::from_str(&response.text().await?)?;
    Ok(data)
}

#[function_component]
pub fn UsageResultsPage() -> Html {
    let sampling_time: UseStateHandle<SamplingTime> = use_state(|| SamplingTime::default());
    let page_state: UseStateHandle<PageState> = use_state(|| PageState::Loading);

    {
        let sampling_time = sampling_time.clone();
        let page_state = page_state.clone();
        use_effect_with(sampling_time.clone(), move |_| {
            let sampling_time = *sampling_time;
            yew::platform::spawn_local(async move {
                // Fetch usage data from the backend
                let result = fetch_usage_data(sampling_time).await;
                match result {
                    Ok(data) => page_state.set(PageState::Loaded(data)),
                    Err(err) => page_state.set(PageState::Error(err.to_string())),
                }
            });
        });
    }

    let content = match &*page_state {
        PageState::Loading => html! { <Spinner /> },
        PageState::Error(msg) => html! { <ErrorMessage msg={msg.clone()} /> },
        PageState::Loaded(hash) => {
            html! { <UsageContainer sampling_time={*sampling_time} data={hash.clone()} /> }
        }
    };

    html! {
        <div>
            {content}
        </div>
    }
}

#[derive(Properties, PartialEq, Clone)]
struct UsageProps {
    pub sampling_time: SamplingTime,
    pub data: HashMap<i64, i64>,
}

#[function_component]
fn UsageContainer(
    UsageProps {
        sampling_time,
        data,
    }: &UsageProps,
) -> Html {
    html! {
        <div>
            <h2>{ "Usage Data" }</h2>
            <p>{ format!("Sampling Time: {:?}", sampling_time) }</p>
            <ul>
                { for data.iter().map(|(time_bucket, total_requests)| {
                    html! {
                        <li>{ format!("Time Bucket: {}, Total Requests: {}", time_bucket, total_requests) }</li>
                    }
                }) }
            </ul>
        </div>
    }
}
