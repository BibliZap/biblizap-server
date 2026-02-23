use std::collections::HashSet;
use std::ops::Deref;
use std::rc::Rc;
use std::{cell::RefCell, ops::DerefMut};

use yew::prelude::*;

pub mod article;
pub use article::Article;

mod filter;
use filter::Filters;

mod footer;
use footer::TableFooter;

mod download;
use download::*;

mod row;
use row::*;

mod card;
use card::CardView;

/// Enum representing the sort state of a column.
#[derive(Clone, Copy, PartialEq, Debug)]
enum SortState {
    None,
    Ascending,
    Descending,
}

impl SortState {
    fn next(&self) -> Self {
        match self {
            SortState::None => SortState::Ascending,
            SortState::Ascending => SortState::Descending,
            SortState::Descending => SortState::None,
        }
    }

    fn icon(&self) -> &'static str {
        match self {
            SortState::None => "bi-chevron-expand",
            SortState::Ascending => "bi-chevron-up",
            SortState::Descending => "bi-chevron-down",
        }
    }
}

/// Enum representing the status of the search results.
#[derive(Clone, PartialEq)]
pub enum ResultsStatus {
    NotRequested,
    Requested,
    RequestError(String),
    Available(Rc<RefCell<Vec<Article>>>),
}

/// Properties for the ResultsContainer component.
#[derive(Clone, PartialEq, Properties)]
pub struct ResultsContainerProps {
    pub results_status: UseStateHandle<ResultsStatus>,
}
/// Container component for displaying search results.
/// Renders a spinner, error message, or the results table based on the `results_status`.
#[function_component(ResultsContainer)]
pub fn table_container(props: &ResultsContainerProps) -> Html {
    let content = match props.results_status.deref() {
        ResultsStatus::NotRequested => {
            html! {}
        }
        ResultsStatus::Available(articles) => {
            html! {<Results articles={articles}/>}
        }
        ResultsStatus::Requested => {
            html! {<Spinner/>}
        }
        ResultsStatus::RequestError(msg) => {
            html! {<Error msg={msg.to_owned()}/>}
        }
    };

    content
}

/// Component for displaying a loading spinner.
#[function_component(Spinner)]
pub fn spinner() -> Html {
    html! {
        <div class="container-fluid">
            <div class="d-flex justify-content-center">
                <div class="spinner-border" role="status" style="width: 5rem; height: 5rem; margin-bottom: 50px;">
                    <span class="visually-hidden">{"Loading..."}</span>
                </div>
            </div>
        </div>
    }
}

/// Properties for the Results (Table) component.
#[derive(Clone, PartialEq, Properties)]
pub struct TableProps {
    articles: Rc<RefCell<Vec<Article>>>,
}

/// Component for displaying the search results in a table.
/// Includes global search, column sorting, column filtering, pagination, and download options.
#[function_component(Results)]
pub fn results(props: &TableProps) -> Html {
    let selected_articles = use_state(|| Rc::new(RefCell::new(HashSet::<String>::new())));

    let update_selected = {
        let selected_articles = selected_articles.clone();
        Callback::from(move |element: (String, bool)| {
            let current_selected = (*selected_articles).clone();
            if element.1 {
                current_selected.borrow_mut().insert(element.0);
            } else {
                current_selected.borrow_mut().remove(&element.0);
            }
            selected_articles.set(current_selected);
        })
    };

    let articles = props.articles.to_owned();
    let global_filter = use_state(|| "".to_string());
    let filters = use_mut_ref(Filters::default);
    let filters = use_state(|| filters);

    // Track sort state for each sortable column
    let sort_year = use_state(|| SortState::None);
    let sort_citations = use_state(|| SortState::None);
    let sort_score = use_state(|| SortState::None);

    let articles_to_display = articles
        .deref()
        .borrow()
        .iter()
        .filter(|a| a.matches_global(&global_filter))
        .filter(|a| a.matches(&filters.deref().borrow()))
        .cloned()
        .collect::<Vec<_>>();

    // Helper function to get articles to download
    let get_articles_to_download = {
        let articles = articles.clone();
        let selected_articles = selected_articles.clone();
        move || -> Vec<Article> {
            if selected_articles.borrow().is_empty() {
                // If nothing selected, return all articles
                articles.deref().borrow().clone()
            } else {
                // Return only selected articles
                articles
                    .deref()
                    .borrow()
                    .iter()
                    .filter(|article| {
                        article
                            .doi
                            .as_ref()
                            .map(|doi| selected_articles.borrow().contains(doi))
                            .unwrap_or(false)
                    })
                    .cloned()
                    .collect()
            }
        }
    };

    let on_excel_download_click = {
        let get_articles = get_articles_to_download.clone();
        let articles = articles.clone();
        Callback::from(move |_: MouseEvent| {
            let articles_to_download = get_articles();
            let bytes = to_excel(&articles_to_download).unwrap();
            let timestamp = chrono::Local::now().to_rfc3339();
            let suffix = if articles_to_download.len() == articles.deref().borrow().len() {
                "all"
            } else {
                "selected"
            };

            match download_bytes_as_file(&bytes, &format!("BibliZap-{suffix}-{timestamp}.xlsx")) {
                Ok(_) => (),
                Err(error) => {
                    gloo_console::log!(format!("{error}"));
                }
            }
        })
    };

    let on_ris_download_click = {
        let get_articles = get_articles_to_download.clone();
        let articles = articles.clone();
        Callback::from(move |_: MouseEvent| {
            let articles_to_download = get_articles();
            let bytes = to_ris(&articles_to_download).unwrap();
            let timestamp = chrono::Local::now().to_rfc3339();
            let suffix = if articles_to_download.len() == articles.deref().borrow().len() {
                "all"
            } else {
                "selected"
            };

            match download_bytes_as_file(&bytes, &format!("BibliZap-{suffix}-{timestamp}.ris")) {
                Ok(_) => (),
                Err(error) => {
                    gloo_console::log!(format!("{error}"));
                }
            }
        })
    };

    let on_bibtex_download_click = {
        let get_articles = get_articles_to_download.clone();
        let articles = articles.clone();
        Callback::from(move |_: MouseEvent| {
            let articles_to_download = get_articles();
            let bytes = to_bibtex(&articles_to_download).unwrap();
            let timestamp = chrono::Local::now().to_rfc3339();
            let suffix = if articles_to_download.len() == articles.deref().borrow().len() {
                "all"
            } else {
                "selected"
            };

            match download_bytes_as_file(&bytes, &format!("BibliZap-{suffix}-{timestamp}.bib")) {
                Ok(_) => (),
                Err(error) => {
                    gloo_console::log!(format!("{error}"));
                }
            }
        })
    };

    let articles_per_page = use_state(|| 10i32);
    let table_current_page = use_state(|| 0i32);

    let first_article = (table_current_page.deref() * articles_per_page.deref())
        .clamp(0, articles_to_display.len() as i32) as usize;
    let last_article = (first_article as i32 + articles_per_page.deref())
        .clamp(0, articles_to_display.len() as i32) as usize;
    let articles_slice = &articles_to_display[first_article..last_article];

    let trigger_update = use_force_update();
    let redraw_table = {
        Callback::from(move |_: ()| {
            trigger_update.force_update();
        })
    };

    html! {
        <div id="table" class="container-fluid">
            <hr/>
            <TableGlobalSearch filter={global_filter.clone()}/>

            // Desktop view - Table
            <div class="d-none d-md-block">
            <table class="table table-hover table-bordered" style="table-layout:fixed">
                <thead>
                    <tr>
                        <th style="width:2%"></th>
                        <HeaderCellDoi articles={articles.clone()} redraw_table={redraw_table.clone()} style=""/>
                        <HeaderCellTitle articles={articles.clone()} redraw_table={redraw_table.clone()} style="width:20%"/>
                        <HeaderCellJournal articles={articles.clone()} redraw_table={redraw_table.clone()} style=""/>
                        <HeaderCellFirstAuthor articles={articles.clone()} redraw_table={redraw_table.clone()} style=""/>
                        <HeaderCellYearPublished articles={articles.clone()} redraw_table={redraw_table.clone()} style="" sort_state={Some(sort_year.clone())}/>
                        <HeaderCellSummary articles={articles.clone()} redraw_table={redraw_table.clone()} style="width:30%"/>
                        <HeaderCellCitations articles={articles.clone()} redraw_table={redraw_table.clone()} style="" sort_state={Some(sort_citations.clone())}/>
                        <HeaderCellScore articles={articles.clone()} redraw_table={redraw_table.clone()} style="" sort_state={Some(sort_score.clone())}/>
                    </tr>
                </thead>
                <thead>
                    <tr>
                        <th></th>
                        <HeaderCellSearchDoi filters={filters.clone()} redraw_table={redraw_table.clone()}/>
                        <HeaderCellSearchTitle filters={filters.clone()} redraw_table={redraw_table.clone()}/>
                        <HeaderCellSearchJournal filters={filters.clone()} redraw_table={redraw_table.clone()}/>
                        <HeaderCellSearchFirstAuthor filters={filters.clone()} redraw_table={redraw_table.clone()}/>
                        <HeaderCellSearchYearPublished filters={filters.clone()} redraw_table={redraw_table.clone()}/>
                        <HeaderCellSearchSummary filters={filters.clone()} redraw_table={redraw_table.clone()}/>
                        <HeaderCellSearchCitations filters={filters.clone()} redraw_table={redraw_table.clone()}/>
                        <HeaderCellSearchScore filters={filters.clone()} redraw_table={redraw_table.clone()}/>
                    </tr>
                </thead>
                <tbody class="table-group-divider">
                    { articles_slice.iter().map(|article| html!{<Row article={article.clone()} update_selected={update_selected.clone()} selected_articles={(*selected_articles).clone()}/>} ).collect::<Html>() }
                </tbody>
            </table>
            <TableFooter article_total_number={articles_to_display.len()} articles_per_page={articles_per_page.clone()} table_current_page={table_current_page.clone()}/>
            </div>

            // Mobile view - Cards
            <div class="d-block d-md-none">
                <CardView
                    articles={articles_slice.to_vec()}
                    update_selected={update_selected.clone()}
                    selected_articles={(*selected_articles).clone()}
                    articles_ref={articles.clone()}
                    redraw={redraw_table.clone()}
                />
                <TableFooter article_total_number={articles_to_display.len()} articles_per_page={articles_per_page.clone()} table_current_page={table_current_page.clone()}/>
            </div>
            <div style="display: flex; gap: 1rem; align-items: center;">
                <h5>{
                    if selected_articles.borrow().is_empty() {
                        "Download everything as:".to_string()
                    } else {
                        format!("Download {} selected articles as:", selected_articles.borrow().len())
                    }
                }</h5>
                <DownloadButton onclick={on_excel_download_click} label="Excel"/>
                <DownloadButton onclick={on_ris_download_click} label="RIS"/>
                <DownloadButton onclick={on_bibtex_download_click} label="BibTeX"/>
            </div>
        </div>
    }
}

/// Properties for table header cells that support sorting.
#[derive(Clone, PartialEq, Properties)]
struct HeaderCellProps {
    articles: Rc<RefCell<Vec<Article>>>,
    redraw_table: Callback<()>,
    style: AttrValue,
    #[prop_or(None)]
    sort_state: Option<UseStateHandle<SortState>>,
}

/// Properties for table header cells that support filtering.
#[derive(Clone, PartialEq, Properties)]
struct HeaderCellSearchProps {
    filters: UseStateHandle<Rc<RefCell<Filters>>>,
    redraw_table: Callback<()>,
}

use paste::paste;

/// Macro to generate header cell components without sorting (label only).
macro_rules! header_cell_no_sort {
    ($field:ident) => {
        paste! {
            /// Table header cell for the '[<$field:snake>]' field, without sorting.
            #[function_component]
            fn [<HeaderCell $field:camel>](props: &HeaderCellProps) -> Html {
                html! {
                    <th class="text-start" style={props.style.clone()}>
                        <strong>{inflections::case::to_title_case(&stringify!{[<$field:snake>]})}</strong>
                    </th>
                }
            }

            /// Table header cell for the '[<$field:snake>]' field, supporting filtering.
            #[function_component]
            fn [<HeaderCellSearch $field:camel>](props: &HeaderCellSearchProps) -> Html {
                let input_node_ref = use_node_ref();
                let oninput = {
                    let filters = props.filters.clone();
                    let input_node_ref = input_node_ref.clone();
                    let redraw_table = props.redraw_table.clone();
                    Callback::from(move |_: InputEvent| {
                        let rc = filters.deref().to_owned();
                        let value = input_node_ref.cast::<web_sys::HtmlInputElement>().unwrap().value();
                        rc.deref().borrow_mut().$field = value.as_str().into();
                        redraw_table.emit(())
                    })
                };

                html! {
                    <th><div class="form-check ps-0"><input type="text" class="form-control" oninput={oninput} ref={input_node_ref}/></div></th>
                }
            }
        }
    }
}

/// Macro to generate header cell components for sorting and filtering.
macro_rules! header_cell {
    ($field:ident) => {
        paste! {
            /// Table header cell for the '[<$field:snake>]' field, supporting sorting.
            #[function_component]
            fn [<HeaderCell $field:camel>](props: &HeaderCellProps) -> Html {
                let onclick = if let Some(sort_state) = &props.sort_state {
                    let articles = props.articles.clone();
                    let redraw_table = props.redraw_table.clone();
                    let sort_state = sort_state.clone();

                    Some(Callback::from(move |_: MouseEvent| {
                        let new_state = (*sort_state).next();
                        sort_state.set(new_state);

                        let mut ref_vec = articles.deref().borrow_mut();
                        match new_state {
                            SortState::None => {
                                // Return to original order (by score desc)
                                ref_vec.deref_mut().sort_by_key(|a| std::cmp::Reverse(a.score.clone().unwrap_or_default()));
                            },
                            SortState::Ascending => {
                                ref_vec.deref_mut().sort_by_key(|a| a.$field.clone().unwrap_or_default());
                            },
                            SortState::Descending => {
                                ref_vec.deref_mut().sort_by_key(|a| std::cmp::Reverse(a.$field.clone().unwrap_or_default()));
                            },
                        }
                        redraw_table.emit(());
                    }))
                } else {
                    None
                };

                let sort_icon = props.sort_state.as_ref().map(|s| (**s).icon()).unwrap_or("");
                let classes = if props.sort_state.is_some() { "sortable-header" } else { "" };

                html! {
                    <th class={classes!("text-start", classes)} style={props.style.clone()} onclick={onclick}>
                        <div class="d-flex align-items-center gap-1">
                            <strong>{inflections::case::to_title_case(&stringify!{[<$field:snake>]})}</strong>
                            if !sort_icon.is_empty() {
                                <i class={classes!("bi", sort_icon)}></i>
                            }
                        </div>
                    </th>
                }
            }

            /// Table header cell for the '[<$field:snake>]' field, supporting filtering.
            #[function_component]
            fn [<HeaderCellSearch $field:camel>](props: &HeaderCellSearchProps) -> Html {
                let input_node_ref = use_node_ref();
                let oninput = {
                    let filters = props.filters.clone();
                    let input_node_ref = input_node_ref.clone();
                    let redraw_table = props.redraw_table.clone();
                    Callback::from(move |_: InputEvent| {
                        let rc = filters.deref().to_owned();
                        let value = input_node_ref.cast::<web_sys::HtmlInputElement>().unwrap().value();
                        rc.deref().borrow_mut().$field = value.as_str().into();
                        redraw_table.emit(())
                    })
                };

                html! {
                    <th><div class="form-check ps-0"><input type="text" class="form-control" oninput={oninput} ref={input_node_ref}/></div></th>
                }
            }
        }
    }
}

// Headers without sorting
header_cell_no_sort!(doi);
header_cell_no_sort!(title);
header_cell_no_sort!(summary);
header_cell_no_sort!(journal);
header_cell_no_sort!(first_author);

// Headers with sorting
header_cell!(year_published);
header_cell!(citations);
header_cell!(score);

/// Properties for the TableGlobalSearch component.
#[derive(Clone, PartialEq, Properties)]
pub struct TableGlobalSearchProps {
    filter: UseStateHandle<String>,
}

/// Component for the global search input above the table.
#[function_component(TableGlobalSearch)]
fn table_global_filter(props: &TableGlobalSearchProps) -> Html {
    let input_node_ref = use_node_ref();
    let oninput = {
        let filter = props.filter.clone();
        let input_node_ref = input_node_ref.clone();
        Callback::from(move |_: InputEvent| {
            let value = input_node_ref
                .cast::<web_sys::HtmlInputElement>()
                .unwrap()
                .value();
            filter.set(value);
        })
    };

    html! {
        <div class="row justify-content-end">
            <div class="mb-3 form-check col-12 col-md-4 col-lg-3">
                <label class="form-label"><strong>{"Search all fields"}</strong></label>
                <input type="text" class="form-control form-control-lg" oninput={oninput} ref={input_node_ref}/>
            </div>
        </div>
    }
}

/// Properties for the Error component.
#[derive(Clone, PartialEq, Properties)]
pub struct ErrorProps {
    msg: AttrValue,
}

/// Component for displaying an error message.
#[function_component(Error)]
pub fn error(props: &ErrorProps) -> Html {
    html! {
        <div class="container-fluid">
            <div class="alert alert-danger" role="alert">
                {props.msg.to_owned()}
            </div>
        </div>
    }
}
