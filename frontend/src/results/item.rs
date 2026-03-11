use std::{cell::RefCell, collections::HashSet, rc::Rc};
use yew::prelude::*;

use super::Article;

/// Properties for a list item component.
#[derive(Clone, PartialEq, Properties)]
pub struct ItemProps {
    pub article: Article,
    pub index: usize,
    pub update_selected: Callback<(String, bool)>,
    pub selected_articles: Rc<RefCell<HashSet<String>>>,
}

/// Component for a single item in the modern results list.
/// Displays article information in a clean, PubMed-inspired layout.
#[function_component(Item)]
pub fn item(props: &ItemProps) -> Html {
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
            let checked = event
                .target_unchecked_into::<web_sys::HtmlInputElement>()
                .checked();
            if let Some(doi) = &doi {
                update_selected.emit((doi.clone(), checked))
            }
        })
    };

    let toggle_expanded = {
        let expanded = expanded.clone();
        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            e.stop_propagation();
            expanded.set(!*expanded);
        })
    };

    let index_str = format!("{}.", props.index + 1);

    html! {
        <div class="py-4 border-bottom" style="transition: background-color 0.15s ease-in-out;">
            <div class="row gx-3">
                // Left Column: Index & Checkbox
                <div class="col-auto d-flex flex-column align-items-center" style="width: 48px;">
                    <div class="form-check mb-1">
                        <input
                            class="form-check-input flex-shrink-0"
                            type="checkbox"
                            checked={is_selected}
                            onchange={onchange}
                            style="width: 1.25em; height: 1.25em; cursor: pointer;"
                        />
                    </div>
                    <span class="text-muted small fw-medium">{index_str}</span>
                </div>

                // Right Column: Article Content
                <div class="col">
                    // Title as prominent link
                    <h5 class="mb-1 lh-base">
                        <a href={doi_link(props.article.doi.clone())} class="text-decoration-none text-primary fw-bold" target="_blank" style="font-size: 1.3rem;">
                            {props.article.title.clone().unwrap_or_else(|| "Untitled Article".to_string())}
                        </a>
                    </h5>

                    // Authors
                    <div class="mb-2 text-dark" style="font-size: 0.95rem;">
                        {props.article.first_author.clone().unwrap_or_else(|| "Unknown Author".to_string())}
                        <span class="text-muted">{" et al."}</span>
                    </div>

                    // Publication Info & Badges
                    <div class="d-flex flex-wrap align-items-center gap-2 mb-2 text-muted small">
                        {if let Some(journal) = &props.article.journal {
                            html! { <span class="fw-medium text-success">{journal}</span> }
                        } else {
                            html! {}
                        }}
                        
                        {if props.article.journal.is_some() && props.article.year_published.is_some() {
                            html! { <span>{" • "}</span> }
                        } else {
                            html! {}
                        }}

                        {if let Some(year) = props.article.year_published {
                            html! { <span>{year}</span> }
                        } else {
                            html! {}
                        }}

                        {if let Some(doi) = &props.article.doi {
                            html! { 
                                <>
                                    <span>{" • "}</span>
                                    <span>{"doi: "}{doi}</span>
                                </>
                            }
                        } else {
                            html! {}
                        }}

                        // Badges aligned to the right or just trailing
                        <div class="ms-md-auto d-flex gap-2 mt-2 mt-md-0">
                            {if let Some(citations) = props.article.citations {
                                html! { 
                                    <span class="badge rounded-pill bg-light text-dark border align-items-center d-flex gap-1" title="Citations">
                                        <i class="bi bi-quote small"></i> {citations}
                                    </span> 
                                }
                            } else {
                                html! {}
                            }}
                            
                            {if let Some(score) = props.article.score {
                                html! { 
                                    <span class="badge rounded-pill bg-light text-dark border align-items-center d-flex gap-1" title="Score">
                                        <i class="bi bi-star-fill text-warning small"></i> {score}
                                    </span> 
                                }
                            } else {
                                html! {}
                            }}
                        </div>
                    </div>

                    // Abstract
                    {if let Some(summary) = &props.article.summary {
                        let is_long = summary.len() > 250;
                        if !*expanded && is_long {
                            let truncated: String = summary.chars().take(250).collect();
                            html! {
                                <div class="text-secondary" style="font-size: 0.95rem; line-height: 1.5;">
                                    <span class="fw-semibold text-dark">{"ABSTRACT: "}</span>
                                    {truncated}{"..."}
                                    <button class="btn btn-link btn-sm p-0 ms-1 text-decoration-none" onclick={toggle_expanded.clone()}>
                                        {"Show more"} <i class="bi bi-chevron-down ms-1"></i>
                                    </button>
                                </div>
                            }
                        } else {
                            html! {
                                <div class="text-secondary" style="font-size: 0.95rem; line-height: 1.5;">
                                    <span class="fw-semibold text-dark">{"ABSTRACT: "}</span>
                                    {summary}
                                    {if is_long {
                                        html! {
                                            <button class="btn btn-link btn-sm p-0 ms-1 text-decoration-none" onclick={toggle_expanded}>
                                                {"Show less"} <i class="bi bi-chevron-up ms-1"></i>
                                            </button>
                                        }
                                    } else {
                                        html! {}
                                    }}
                                </div>
                            }
                        }
                    } else {
                        html! {}
                    }}
                </div>
            </div>
        </div>
    }
}
