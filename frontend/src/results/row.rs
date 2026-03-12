use super::Article;
use std::{cell::RefCell, collections::HashSet, rc::Rc};
use yew::prelude::*;

/// Properties for a table row component.
#[derive(Clone, PartialEq, Properties)]
pub struct RowProps {
    pub article: Article,
    pub update_selected: Callback<(String, bool)>,
    pub selected_articles: Rc<RefCell<HashSet<String>>>,
}
/// Component for a single row in the results table.
/// Displays article information and a checkbox for selection.
#[function_component(Row)]
pub fn row(props: &RowProps) -> Html {
    let expanded = use_state(|| false);

    fn doi_link(doi: Option<String>) -> Option<String> {
        let doi = doi?;
        Some(format!("https://doi.org/{}", doi))
    }

    let is_selected = props
        .article
        .doi
        .as_ref()
        .map(|doi| props.selected_articles.borrow().contains(doi))
        .unwrap_or(false);

    let onchange = {
        let update_selected = props.update_selected.clone();
        let doi = props.article.doi.clone();
        Callback::from(move |event: Event| {
            event.stop_propagation();
            let update_selected = update_selected.clone();
            let doi = doi.clone();
            let checked = event
                .target_unchecked_into::<web_sys::HtmlInputElement>()
                .checked();
            if let Some(doi) = doi {
                update_selected.emit((doi, checked))
            }
        })
    };

    let toggle_expanded = {
        let expanded = expanded.clone();
        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            expanded.set(!*expanded);
        })
    };

    html! {
        <>
            <tr class="cursor-pointer" onclick={toggle_expanded} style="cursor: pointer; transition: background-color 0.15s ease-in-out;">
                <td onclick={Callback::from(|e: MouseEvent| e.stop_propagation())} style="text-align: center; vertical-align: middle;">
                    <input type={"checkbox"} class={"row-checkbox"} checked={is_selected} onchange={onchange}/>
                </td>
                <td class="fw-bold" style="word-wrap: break-word">
                    {props.article.title.clone().unwrap_or_default()}
                </td>
                <td style="word-wrap: break-word">{props.article.journal.clone().unwrap_or_default()}</td>
                <td>{props.article.first_author.clone().unwrap_or_default()}</td>
                <td>{props.article.year_published.unwrap_or_default()}</td>
                <td>{props.article.citations.unwrap_or_default()}</td>
                <td>
                    <div class="d-flex justify-content-between align-items-center">
                        <span>{props.article.score.unwrap_or_default()}</span>
                        <i class={classes!("bi", if *expanded { "bi-chevron-up" } else { "bi-chevron-down" })}></i>
                    </div>
                </td>
            </tr>
            {
                if *expanded {
                    html! {
                        <tr class="table-light">
                            <td colspan="7">
                                <div class="px-4 py-3">
                                    {if let Some(summary) = &props.article.summary {
                                        html! { <p class="mb-3 text-secondary" style="font-size: 0.95em;"><strong>{"Abstract:"}</strong>{" "}{summary}</p> }
                                    } else {
                                        html! {}
                                    }}

                                    <div class="d-flex gap-2">
                                        {if let Some(doi) = &props.article.doi {
                                            html! {
                                                <a href={doi_link(Some(doi.clone()))} target="_blank" class="btn btn-sm btn-outline-primary">
                                                    <i class="bi bi-box-arrow-up-right"></i> {" View on Publisher Site"}
                                                </a>
                                            }
                                        } else {
                                            html! {
                                                <span class="text-muted small">{"No DOI available"}</span>
                                            }
                                        }}
                                    </div>
                                </div>
                            </td>
                        </tr>
                    }
                } else {
                    html! {}
                }
            }
        </>
    }
}
