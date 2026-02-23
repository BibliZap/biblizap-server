use std::ops::Deref;

use wasm_bindgen::JsCast;
use web_sys::HtmlElement;
use yew::prelude::*;

/// Properties for the TableFooter component.
#[derive(Clone, PartialEq, Properties)]
pub struct TableFooterProps {
    pub article_total_number: usize,
    pub articles_per_page: UseStateHandle<i32>,
    pub table_current_page: UseStateHandle<i32>,
}

/// Component for the table footer, including pagination and articles per page dropdown.
#[function_component(TableFooter)]
pub fn table_footer(props: &TableFooterProps) -> Html {
    let table_current_page = props.table_current_page.deref().to_owned();
    let articles_per_page = props.articles_per_page.deref().to_owned();
    let first_article = table_current_page * articles_per_page + 1;
    let last_article = first_article + articles_per_page - 1;

    let total_page_number = (props.article_total_number as i32) / articles_per_page;
    let last_page_index = total_page_number - 1;

    let contiguous_window_radius = 2;
    let contiguous_low_bound =
        (table_current_page - contiguous_window_radius).clamp(0, total_page_number);
    let contiguous_high_bound =
        (contiguous_low_bound + 2 * contiguous_window_radius + 1).clamp(0, total_page_number);
    let contiguous_low_bound =
        (contiguous_high_bound - 2 * contiguous_window_radius - 1).clamp(0, total_page_number); //Recalculate in case high bound got clamped

    let contiguous_range = contiguous_low_bound..contiguous_high_bound;

    html! {
        <div class="row py-2" id="table_footer">
            <div class="col">
                <div role="status" aria-live="polite">{format!("Showing {} to {} of {} entries", first_article, std::cmp::min(last_article as usize, props.article_total_number), props.article_total_number)}</div>
                <ArticlesPerPageDropdown articles_per_page={props.articles_per_page.clone()} table_current_page={props.table_current_page.clone()}/>
            </div>


            <div class="col">
                <div class="float-end">
                    <ul class="pagination pagination-lg pagination-sm-mobile">
                        if contiguous_low_bound != 0 {
                            <PageItem table_current_page={props.table_current_page.clone()} page_index={0}/>
                            if contiguous_low_bound > 1 {
                                <li class="page-item disabled">
                                    <a aria-disabled="true" role="link" tabindex="-1" class="page-link">{"…"}</a>
                                </li>
                            }
                        }

                        if contiguous_range.len() != 1 {
                            { contiguous_range.into_iter().map(|index| html!{<PageItem table_current_page={props.table_current_page.clone()} page_index={index}/>} ).collect::<Html>() }
                        }

                        if contiguous_high_bound != total_page_number {
                            if total_page_number-contiguous_low_bound > 1 {
                                <li class="page-item disabled"><a aria-disabled="true" role="link" tabindex="-1" class="page-link">{"…"}</a></li>
                            }
                            <PageItem table_current_page={props.table_current_page.clone()} page_index={last_page_index}/>
                        }
                    </ul>
                </div>
            </div>
        </div>
    }
}

/// Properties for the ArticlesPerPageDropdown component.
#[derive(Clone, PartialEq, Properties)]
struct ArticlesPerPageDropdownProps {
    articles_per_page: UseStateHandle<i32>,
    table_current_page: UseStateHandle<i32>,
}
/// Component for the dropdown to select the number of articles displayed per page.
#[function_component(ArticlesPerPageDropdown)]
fn articles_per_page_dropdown(props: &ArticlesPerPageDropdownProps) -> Html {
    html! {
        <div class="dropdown">
            <button class="btn btn-outline-secondary dropdown-toggle" type="button" data-bs-toggle="dropdown" aria-expanded="false">
                {"Articles per page"}
            </button>

            <ul class="dropdown-menu">
                <ArticlesPerPageDropdownItem table_articles_per_page={props.articles_per_page.clone()} table_current_page={props.table_current_page.clone()} value=10/>
                <ArticlesPerPageDropdownItem table_articles_per_page={props.articles_per_page.clone()} table_current_page={props.table_current_page.clone()} value=50/>
                <ArticlesPerPageDropdownItem table_articles_per_page={props.articles_per_page.clone()} table_current_page={props.table_current_page.clone()} value=100/>
                <ArticlesPerPageDropdownItem table_articles_per_page={props.articles_per_page.clone()} table_current_page={props.table_current_page.clone()} value=500/>
            </ul>
        </div>
    }
}

/// Properties for an item in the ArticlesPerPageDropdown.
#[derive(Clone, PartialEq, Properties)]
struct ArticlesPerPageDropdownItemProps {
    table_articles_per_page: UseStateHandle<i32>,
    table_current_page: UseStateHandle<i32>,
    value: i32,
}

/// Component for a single item in the articles per page dropdown.
#[function_component(ArticlesPerPageDropdownItem)]
fn articles_per_page_dropdown(props: &ArticlesPerPageDropdownItemProps) -> Html {
    let onclick = {
        let articles_per_page = props.table_articles_per_page.clone();
        let table_current_page = props.table_current_page.clone();
        let value = props.value;
        Callback::from(move |event: MouseEvent| {
            table_current_page.set(0);
            articles_per_page.set(value);

            event.prevent_default();
            let element = gloo_utils::document()
                .get_element_by_id("table")
                .and_then(|element| element.dyn_into::<HtmlElement>().ok());
            if let Some(element) = element {
                element.scroll_into_view();
            }
        })
    };

    html! {
        <li><a class="dropdown-item" {onclick}>{props.value}</a></li>
    }
}

/// Properties for a pagination page item.
#[derive(Clone, PartialEq, Properties)]
struct PageItemProps {
    table_current_page: UseStateHandle<i32>,
    page_index: i32,
}
/// Component for a single page number button in the pagination control.
#[function_component(PageItem)]
fn page_item(props: &PageItemProps) -> Html {
    let onclick = {
        let table_current_page = props.table_current_page.clone();
        let page_index = props.page_index;
        Callback::from(move |event: MouseEvent| {
            table_current_page.set(page_index);

            event.prevent_default();
            let element = gloo_utils::document()
                .get_element_by_id("table")
                .and_then(|element| element.dyn_into::<HtmlElement>().ok());
            if let Some(element) = element {
                element.scroll_into_view();
            }
        })
    };

    let class = match *props.table_current_page.deref() == props.page_index {
        true => "page-item active",
        false => "page-item",
    };

    html! {
        <li class={class}><button class="page-link " {onclick}>{props.page_index+1}</button></li>
    }
}
