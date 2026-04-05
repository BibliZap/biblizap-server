use std::cell::RefCell;
use std::rc::Rc;

use yew::prelude::*;

use crate::results::Article;

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum SortState {
    None,
    Ascending,
    Descending,
}

impl SortState {
    fn next(self) -> Self {
        match self {
            SortState::None => SortState::Ascending,
            SortState::Ascending => SortState::Descending,
            SortState::Descending => SortState::None,
        }
    }

    fn icon(self) -> &'static str {
        match self {
            SortState::None => "bi-chevron-expand",
            SortState::Ascending => "bi-chevron-up",
            SortState::Descending => "bi-chevron-down",
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
    sort_state: SortState,
    on_sort: Callback<SortState>,
    icon: &'static str,
    label: &'static str,
}

#[function_component]
fn SortButton(props: &SortButtonProps) -> Html {
    let on_click = {
        let sort_state = props.sort_state;
        let on_sort = props.on_sort.clone();
        Callback::from(move |_: MouseEvent| on_sort.emit(sort_state.next()))
    };
    html! {
        <button
            class={classes!("btn", "btn-sm", if props.sort_state != SortState::None { "btn-primary" } else { "btn-outline-secondary" })}
            onclick={on_click}
        >
            <i class={classes!("bi", props.icon, "me-1")}></i>
            {" "}{&props.label}
            {if props.sort_state != SortState::None { html! { <i class={classes!("bi", props.sort_state.icon())}></i> } } else { html!{} }}
        </button>
    }
}

#[derive(Clone, PartialEq, Properties)]
pub struct SortButtonsProps {
    pub articles: Rc<RefCell<Vec<Article>>>,
    pub redraw_table: Callback<()>,
}

#[function_component]
pub fn SortButtons(
    SortButtonsProps {
        articles,
        redraw_table,
    }: &SortButtonsProps,
) -> Html {
    let sort_year = use_state(|| SortState::None);
    let sort_citations = use_state(|| SortState::None);
    let sort_score = use_state(|| SortState::None);

    let on_sort_year = {
        let articles = articles.clone();
        let redraw_table = redraw_table.clone();
        let sort_year = sort_year.clone();
        Callback::from(move |new_state: SortState| {
            sort_year.set(new_state);
            let mut ref_vec = articles.borrow_mut();
            match new_state {
                SortState::None => {
                    ref_vec.sort_by_key(|a| std::cmp::Reverse(a.score.unwrap_or_default()))
                }
                SortState::Ascending => {
                    ref_vec.sort_by_key(|a| a.year_published.unwrap_or_default())
                }
                SortState::Descending => {
                    ref_vec.sort_by_key(|a| std::cmp::Reverse(a.year_published.unwrap_or_default()))
                }
            }
            redraw_table.emit(());
        })
    };

    let on_sort_citations = {
        let articles = articles.clone();
        let redraw_table = redraw_table.clone();
        let sort_citations = sort_citations.clone();
        Callback::from(move |new_state: SortState| {
            sort_citations.set(new_state);
            let mut ref_vec = articles.borrow_mut();
            match new_state {
                SortState::None => {
                    ref_vec.sort_by_key(|a| std::cmp::Reverse(a.score.unwrap_or_default()))
                }
                SortState::Ascending => ref_vec.sort_by_key(|a| a.citations.unwrap_or_default()),
                SortState::Descending => {
                    ref_vec.sort_by_key(|a| std::cmp::Reverse(a.citations.unwrap_or_default()))
                }
            }
            redraw_table.emit(());
        })
    };

    let on_sort_score = {
        let articles = articles.clone();
        let redraw_table = redraw_table.clone();
        let sort_score = sort_score.clone();
        Callback::from(move |new_state: SortState| {
            sort_score.set(new_state);
            let mut ref_vec = articles.borrow_mut();
            match new_state {
                SortState::None | SortState::Descending => {
                    ref_vec.sort_by_key(|a| std::cmp::Reverse(a.score.unwrap_or_default()))
                }
                SortState::Ascending => ref_vec.sort_by_key(|a| a.score.unwrap_or_default()),
            }
            redraw_table.emit(());
        })
    };

    html! {
        <div class="col-md-7 d-flex justify-content-md-end gap-2 flex-wrap">
            <span class="align-self-center fw-semibold text-secondary me-2">{"Sort by:"}</span>
            <SortButton sort_state={*sort_year} on_sort={on_sort_year} icon="bi-calendar" label="Year" />
            <SortButton sort_state={*sort_citations} on_sort={on_sort_citations} icon="bi-quote" label="Citations" />
            <SortButton sort_state={*sort_score} on_sort={on_sort_score} icon="bi-star-fill" label="Score" />
        </div>
    }
}
