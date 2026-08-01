use yew::prelude::*;

mod data;
use data::*;

mod chart;
use chart::*;

use crate::results::{ErrorMessage, Spinner};

enum PageState {
    Loading,
    Loaded(UsageData),
    Error(String),
}

#[function_component]
pub fn UsageResultsPage() -> Html {
    let page_state: UseStateHandle<PageState> = use_state(|| PageState::Loading);

    {
        let page_state = page_state.clone();
        use_effect_with((), move |_| {
            yew::platform::spawn_local(async move {
                // Fetch usage data from the backend
                let result = fetch_usage_data().await;
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
        PageState::Loaded(usage_data) => {
            html! { <UsageChart data={usage_data.clone()} /> }
        }
    };

    html! {
        <div>
            {content}
        </div>
    }
}
