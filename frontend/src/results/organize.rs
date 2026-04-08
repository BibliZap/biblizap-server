use yew::prelude::*;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum SortByState {
    Ascending,
    Descending,
}

impl SortByState {
    fn next(option: Option<Self>) -> Option<Self> {
        match option {
            None => Some(SortByState::Ascending),
            Some(SortByState::Ascending) => Some(SortByState::Descending),
            Some(SortByState::Descending) => None,
        }
    }

    fn icon(option: Option<Self>) -> &'static str {
        match option {
            None => "bi-chevron-expand",
            Some(SortByState::Ascending) => "bi-chevron-up",
            Some(SortByState::Descending) => "bi-chevron-down",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum SortBy {
    Year(SortByState),
    Citations(SortByState),
    Score(SortByState),
}

impl SortBy {
    fn loose_eq(&self, other: &SortBy) -> bool {
        matches!(
            (self, other),
            (SortBy::Year(_), SortBy::Year(_))
                | (SortBy::Citations(_), SortBy::Citations(_))
                | (SortBy::Score(_), SortBy::Score(_))
        )
    }
}

#[derive(Clone, PartialEq)]
pub struct SortTotalState(pub Vec<SortBy>);

impl SortTotalState {
    pub fn get(&self, kind: &SortBy) -> Option<SortByState> {
        self.0.iter().find(|s| s.loose_eq(kind)).map(|s| match s {
            SortBy::Year(state) | SortBy::Citations(state) | SortBy::Score(state) => *state,
        })
    }

    pub fn toggle(&mut self, kind: SortBy) {
        let current = self.get(&kind);
        self.0.retain(|s| !s.loose_eq(&kind));
        if let Some(s) = SortByState::next(current) {
            let new = match kind {
                SortBy::Year(_) => SortBy::Year(s),
                SortBy::Citations(_) => SortBy::Citations(s),
                SortBy::Score(_) => SortBy::Score(s),
            };
            self.0.insert(0, new);
        }
    }
}

#[derive(Clone, PartialEq, Properties)]
pub struct GlobalFilterProps {
    pub on_name_entry: Callback<String>,
}

#[function_component]
pub fn GlobalFilter(props: &GlobalFilterProps) -> Html {
    html! {
    <div class="col-md-5 mb-3 mb-md-0">
        <div class="input-group">
            <span class="input-group-text bg-body-secondary"><i class="bi bi-search"></i></span>
            <input type="text" class="form-control" placeholder="Search across all fields..." oninput={
                let on_name_entry = props.on_name_entry.clone();
                Callback::from(move |e: InputEvent| {
                    let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                    on_name_entry.emit(input.value());
                })
            } />
        </div>
    </div>
    }
}

#[derive(Clone, PartialEq, Properties)]
pub struct LoadMoreArticlesButtonProps {
    pub n_articles: usize,
    pub display_limit: usize,
    pub update_display_limit: Callback<usize>,
}

#[function_component]
pub fn LoadMoreArticlesButton(
    LoadMoreArticlesButtonProps {
        n_articles,
        display_limit,
        update_display_limit,
    }: &LoadMoreArticlesButtonProps,
) -> Html {
    let on_load_more = {
        let update_display_limit = update_display_limit.clone();
        Callback::from(move |e: MouseEvent| {
            e.prevent_default();

            // Blur the button to prevent the browser from automatically scrolling
            // down to keep the focused button in the viewport.
            use wasm_bindgen::JsCast;
            if let Some(target) = e.target() {
                if let Ok(element) = target.dyn_into::<web_sys::HtmlElement>() {
                    let _ = element.blur();
                }
            }

            update_display_limit.emit(20);
        })
    };

    html! {
        {if *display_limit < *n_articles {
            html! {
                <div class="d-flex justify-content-center my-4">
                    <button class="btn btn-outline-primary rounded-pill px-4 py-2 fw-semibold" onclick={on_load_more}>
                        {"Load More Articles..."}
                    </button>
                </div>
            }
        } else {
            html! {
                <div class="text-center text-muted my-4 py-3 border-top">
                    <small>{"All "}{*n_articles}{" articles displayed."}</small>
                </div>
            }
        }}
    }
}

#[derive(Clone, PartialEq, Properties)]
struct SortButtonProps {
    sort_state: Option<SortByState>,
    on_click: Callback<()>,
    icon: &'static str,
    label: &'static str,
}

#[function_component]
fn SortButton(props: &SortButtonProps) -> Html {
    let on_click = {
        let on_click = props.on_click.clone();
        Callback::from(move |_: MouseEvent| on_click.emit(()))
    };
    html! {
        <button
            class={classes!("btn", "btn-sm", if props.sort_state.is_some() { "btn-primary" } else { "btn-outline-secondary" })}
            onclick={on_click}
        >
            <i class={classes!("bi", props.icon, "me-1")}></i>
            {" "}{&props.label}
            {if props.sort_state.is_some() { html! { <i class={classes!("bi", SortByState::icon(props.sort_state))}></i> } } else { html!{} }}
        </button>
    }
}

#[derive(Clone, PartialEq, Properties)]
pub struct SortButtonsProps {
    pub sort_state: SortTotalState,
    pub on_sort: Callback<SortBy>,
}

#[function_component]
pub fn SortButtons(
    SortButtonsProps {
        sort_state,
        on_sort,
    }: &SortButtonsProps,
) -> Html {
    let year_state = sort_state.get(&SortBy::Year(SortByState::Ascending));
    let citations_state = sort_state.get(&SortBy::Citations(SortByState::Ascending));
    let score_state = sort_state.get(&SortBy::Score(SortByState::Ascending));

    let on_click_year = {
        let on_sort = on_sort.clone();
        Callback::from(move |_: ()| on_sort.emit(SortBy::Year(SortByState::Ascending)))
    };
    let on_click_citations = {
        let on_sort = on_sort.clone();
        Callback::from(move |_: ()| on_sort.emit(SortBy::Citations(SortByState::Ascending)))
    };
    let on_click_score = {
        let on_sort = on_sort.clone();
        Callback::from(move |_: ()| on_sort.emit(SortBy::Score(SortByState::Ascending)))
    };

    html! {
        <div class="col-md-7 d-flex justify-content-md-end gap-2 flex-wrap">
            <span class="align-self-center fw-semibold text-secondary me-2">{"Sort by:"}</span>
            <SortButton sort_state={year_state} on_click={on_click_year} icon="bi-calendar" label="Year" />
            <SortButton sort_state={citations_state} on_click={on_click_citations} icon="bi-quote" label="Citations" />
            <SortButton sort_state={score_state} on_click={on_click_score} icon="bi-star-fill" label="Score" />
        </div>
    }
}
