use std::collections::HashMap;

use yew::prelude::*;

use crate::results::{ErrorMessage, Spinner};

#[derive(Clone, Copy, PartialEq, Debug, thiserror::Error)]
pub enum BinSizeError {
    #[error("Invalid custom days: {0}")]
    InvalidCustomDays(i64),
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum BinSize {
    Daily,
    Weekly,
    Monthly,
    CustomDays(CustomDays),
}

#[derive(Clone, Copy, PartialEq, Debug)]
struct CustomDays(i64);

impl TryFrom<i64> for CustomDays {
    type Error = BinSizeError;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        if value <= 0 {
            return Err(BinSizeError::InvalidCustomDays(value));
        }
        Ok(CustomDays(value))
    }
}

impl From<CustomDays> for i64 {
    fn from(custom_days: CustomDays) -> Self {
        custom_days.0
    }
}

impl BinSize {
    fn get_end_date(&self, start: time::Date) -> time::Date {
        match self {
            BinSize::Daily => start + time::Duration::days(1),
            BinSize::Weekly => start + time::Duration::weeks(1),
            BinSize::CustomDays(days) => start + time::Duration::days(days.0),
            BinSize::Monthly => Self::get_end_day_monthly(start),
        }
    }

    fn get_end_day_monthly(start: time::Date) -> time::Date {
        let year = start.year();
        let month = start.month();
        let next_month = month.next();
        let next_year = if next_month == time::Month::January {
            year + 1
        } else {
            year
        };

        time::Date::from_calendar_date(next_year, next_month, 1).unwrap_or(start)
    }
}

enum PageState {
    Loading,
    Loaded(UsageData),
    Error(String),
}

#[derive(Clone, PartialEq, Debug)]
struct UsageData {
    pub data: HashMap<i32, i64>,
}

impl From<HashMap<i32, i64>> for UsageData {
    fn from(data: HashMap<i32, i64>) -> Self {
        UsageData { data }
    }
}

impl UsageData {
    fn get_total_requests(&self) -> i64 {
        self.data.values().sum()
    }

    fn get_requests_for_julian_date(&self, julian_date: i32) -> i64 {
        self.data.get(&julian_date).copied().unwrap_or(0)
    }

    fn get_requests_for_date(&self, date: time::Date) -> i64 {
        self.get_requests_for_julian_date(date.to_julian_day())
    }

    fn get_total_requests_for_date_until(
        &self,
        start_date: time::Date,
        end_date: time::Date,
    ) -> i64 {
        let mut total = 0;
        let mut current_date = start_date;
        while current_date < end_date {
            total += self.get_requests_for_date(current_date);
            current_date = current_date.next_day().unwrap_or(current_date);
        }
        total
    }
}

struct UsageBinIterator<'a> {
    usage_data: &'a UsageData,
    current_start_date: time::Date,
    bin_size: BinSize,
    end_date: time::Date,
}

impl<'a> Iterator for UsageBinIterator<'a> {
    type Item = (time::Date, time::Date, i64);

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_start_date >= self.end_date {
            return None;
        }

        let bin_end_date = self.bin_size.get_end_date(self.current_start_date);

        let effective_end = std::cmp::min(bin_end_date, self.end_date);

        let total_requests = self
            .usage_data
            .get_total_requests_for_date_until(self.current_start_date, effective_end);

        let current_bin_start = self.current_start_date;

        self.current_start_date = effective_end;

        Some((current_bin_start, effective_end, total_requests))
    }
}

async fn fetch_usage_data() -> Result<UsageData, Box<dyn std::error::Error>> {
    let response = gloo_net::http::Request::post("/api/usage_info/")
        .header("Content-Type", "text/plain")
        .send()
        .await?;
    let data: HashMap<i32, i64> = serde_json::from_str(&response.text().await?)?;
    Ok(data.into())
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
            html! { <UsageContainer data={usage_data.clone()} /> }
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
    pub data: UsageData,
}

#[function_component]
fn UsageContainer(UsageProps { data }: &UsageProps) -> Html {
    let iterator = UsageBinIterator {
        usage_data: data,
        current_start_date: time::OffsetDateTime::now_utc().date() - time::Duration::days(30),
        bin_size: BinSize::Daily,
        end_date: time::OffsetDateTime::now_utc().date() + time::Duration::days(1),
    };

    html! {
        <div>
            <h2>{ "Usage Data" }</h2>
            <ul>
                { for iterator.map(|(start_date, end_date, total_requests)| {
                    html! {
                        <li>{ format!("Time Bucket: {} to {}, Total Requests: {}", start_date, end_date, total_requests) }</li>
                    }
                }) }
            </ul>
        </div>
    }
}
