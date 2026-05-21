use std::collections::HashSet;
use yew::prelude::*;

use super::Article;

/// Properties for a list item component.
#[derive(Clone, PartialEq, Properties)]
pub struct ItemProps {
    pub article: Article,
    pub index: usize,
    pub update_selected: Callback<(String, bool)>,
    pub selected_articles: HashSet<String>,
}

#[derive(Clone, PartialEq, Properties)]
struct ItemMetaProps {
    journal: Option<String>,
    year: Option<i32>,
    doi: Option<String>,
    citations: Option<i32>,
    score: Option<i32>,
}

#[function_component]
fn ItemMeta(props: &ItemMetaProps) -> Html {
    html! {
        <div class="d-flex flex-wrap align-items-center gap-2 mb-2 text-muted small">
            if let Some(journal) = &props.journal {
                <span class="fw-medium text-success">{journal}</span>
            }
            if props.journal.is_some() && props.year.is_some() {
                <span>{" • "}</span>
            }
            if let Some(year) = props.year {
                <span>{year}</span>
            }
            if let Some(doi) = &props.doi {
                <span>{" • "}</span>
                <span>{"doi: "}{doi}</span>
            }
            <div class="ms-md-auto d-flex gap-2 mt-2 mt-md-0">
                if let Some(citations) = props.citations {
                    <span class="badge rounded-pill bg-light text-dark border align-items-center d-flex gap-1" title="Citations">
                        <i class="bi bi-quote small"></i> {citations}
                    </span>
                }
                if let Some(score) = props.score {
                    <span class="badge rounded-pill bg-light text-muted border align-items-center d-flex gap-1" title="Score">
                        <i class="bi bi-star-fill text-warning small"></i> {score}
                    </span>
                }
            </div>
        </div>
    }
}

#[derive(Clone, PartialEq, Properties)]
struct ItemAbstractProps {
    summary: Option<String>,
}

#[function_component]
fn ItemAbstract(props: &ItemAbstractProps) -> Html {
    let expanded = use_state(|| false);

    let Some(summary) = &props.summary else {
        return html! {};
    };

    let is_long = summary.len() > 250;

    let toggle = {
        let expanded = expanded.clone();
        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            e.stop_propagation();
            expanded.set(!*expanded);
        })
    };

    html! {
        <div class="text-secondary" style="font-size: 0.95rem; line-height: 1.5;">
            <span class="fw-semibold text-muted">{"ABSTRACT: "}</span>
            if !*expanded && is_long {
                { summary.chars().take(250).collect::<String>() }{"..."}
                <button class="btn btn-link btn-sm p-0 ms-1 text-decoration-none" onclick={toggle}>
                    {"Show more"} <i class="bi bi-chevron-down ms-1"></i>
                </button>
            } else {
                {summary}
                if is_long {
                    <button class="btn btn-link btn-sm p-0 ms-1 text-decoration-none" onclick={toggle}>
                        {"Show less"} <i class="bi bi-chevron-up ms-1"></i>
                    </button>
                }
            }
        </div>
    }
}

/// Component for a single item in the modern results list.
/// Displays article information in a clean, PubMed-inspired layout.
#[function_component]
pub fn Item(props: &ItemProps) -> Html {
    fn doi_link(doi: Option<String>) -> Option<String> {
        Some(format!("https://doi.org/{}", doi?))
    }

    let is_selected = props
        .article
        .doi
        .as_ref()
        .map(|doi| props.selected_articles.contains(doi))
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

    let onclick_item = {
        let update_selected = props.update_selected.clone();
        let doi = props.article.doi.clone();
        Callback::from(move |_: MouseEvent| {
            if let Some(doi) = &doi {
                update_selected.emit((doi.clone(), !is_selected));
            }
        })
    };

    let stop_click = Callback::from(|e: MouseEvent| e.stop_propagation());

    let index_str = format!("{}.", props.index + 1);
    let item_class = if is_selected {
        "result-item p-4 border-bottom selected"
    } else {
        "result-item p-4 border-bottom"
    };

    html! {
        <div class={item_class} onclick={onclick_item} style="transition: background-color 0.15s ease-in-out;">
            <div class="row gx-3">
                // Left Column: Checkbox
                <div class="col-auto d-flex align-items-start pt-1" style="width: 32px;">
                    <input
                        class="result-item-checkbox"
                        type="checkbox"
                        checked={is_selected}
                        onchange={onchange}
                        onclick={stop_click.clone()}
                    />
                </div>

                // Right Column: Article Content
                <div class="col">
                    <h5 class="mb-1">
                        <span class="text-muted fw-normal me-2" style="font-size: 0.95rem;">{index_str}</span>
                        <a href={doi_link(props.article.doi.clone())} class="article-title-link text-decoration-none text-body fw-semibold" target="_blank" style="font-size: 1.05rem;" onclick={stop_click}>
                            {props.article.title.clone().unwrap_or_else(|| "Untitled Article".to_string())}
                        </a>
                    </h5>
                    <div class="mb-2 text-muted" style="font-size: 0.95rem;">
                        {props.article.first_author.clone().unwrap_or_else(|| "Unknown Author".to_string())}
                        <span class="text-muted">{" et al."}</span>
                    </div>
                    <ItemMeta
                        journal={props.article.journal.clone()}
                        year={props.article.year_published}
                        doi={props.article.doi.clone()}
                        citations={props.article.citations}
                        score={props.article.score}
                    />
                    <ItemAbstract summary={props.article.summary.clone()} />
                </div>
            </div>
        </div>
    }
}
