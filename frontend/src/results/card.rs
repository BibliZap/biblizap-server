use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use yew::prelude::*;

use crate::results::Article;

/// Properties for the CardView component.
#[derive(Clone, PartialEq, Properties)]
pub struct CardViewProps {
    pub articles: Vec<Article>,
    pub update_selected: Callback<(String, bool)>,
    pub selected_articles: Rc<RefCell<HashSet<String>>>,
    pub articles_ref: Rc<RefCell<Vec<Article>>>,
    pub redraw: Callback<()>,
}

/// Component for displaying articles as cards on mobile devices.
#[function_component(CardView)]
pub fn card_view(props: &CardViewProps) -> Html {
    let sort_by_year_desc = {
        let articles_ref = props.articles_ref.clone();
        let redraw = props.redraw.clone();
        Callback::from(move |_: MouseEvent| {
            let mut articles = articles_ref.borrow_mut();
            articles
                .sort_by_key(|a| std::cmp::Reverse(a.year_published.clone().unwrap_or_default()));
            redraw.emit(());
        })
    };

    let sort_by_citations_desc = {
        let articles_ref = props.articles_ref.clone();
        let redraw = props.redraw.clone();
        Callback::from(move |_: MouseEvent| {
            let mut articles = articles_ref.borrow_mut();
            articles.sort_by_key(|a| std::cmp::Reverse(a.citations.unwrap_or_default()));
            redraw.emit(());
        })
    };

    let sort_by_score_desc = {
        let articles_ref = props.articles_ref.clone();
        let redraw = props.redraw.clone();
        Callback::from(move |_: MouseEvent| {
            let mut articles = articles_ref.borrow_mut();
            articles.sort_by_key(|a| std::cmp::Reverse(a.score.unwrap_or_default()));
            redraw.emit(());
        })
    };

    html! {
        <div class="container-fluid">
            // Sort buttons
            <div class="mb-3 d-flex gap-2 flex-wrap">
                <span class="fw-bold align-self-center">{"Sort by:"}</span>
                <button class="btn btn-outline-secondary btn-sm" onclick={sort_by_year_desc}>
                    <i class="bi bi-calendar"></i> {" Year"}
                </button>
                <button class="btn btn-outline-secondary btn-sm" onclick={sort_by_citations_desc}>
                    <i class="bi bi-quote"></i> {" Citations"}
                </button>
                <button class="btn btn-outline-secondary btn-sm" onclick={sort_by_score_desc}>
                    <i class="bi bi-star"></i> {" Score"}
                </button>
            </div>

            <div class="row g-3">
                {props.articles.iter().map(|article| {
                    html! {
                        <div class="col-12">
                            <ArticleCard
                                article={article.clone()}
                                update_selected={props.update_selected.clone()}
                                selected_articles={props.selected_articles.clone()}
                            />
                        </div>
                    }
                }).collect::<Html>()}
            </div>
        </div>
    }
}

/// Properties for individual article card.
#[derive(Clone, PartialEq, Properties)]
struct ArticleCardProps {
    article: Article,
    update_selected: Callback<(String, bool)>,
    selected_articles: Rc<RefCell<HashSet<String>>>,
}

/// Individual article card component.
#[function_component(ArticleCard)]
fn article_card(props: &ArticleCardProps) -> Html {
    let article = &props.article;
    let summary_expanded = use_state(|| false);

    let is_selected = article
        .doi
        .as_ref()
        .map(|doi| props.selected_articles.borrow().contains(doi))
        .unwrap_or(false);

    let toggle_summary = {
        let summary_expanded = summary_expanded.clone();
        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            summary_expanded.set(!*summary_expanded);
        })
    };

    let onchange = {
        let doi = article.doi.clone();
        let update_selected = props.update_selected.clone();
        Callback::from(move |e: Event| {
            if let Some(doi) = &doi {
                let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                update_selected.emit((doi.clone(), input.checked()));
            }
        })
    };

    let doi_link = article
        .doi
        .as_ref()
        .map(|doi| format!("https://doi.org/{}", doi));

    html! {
        <div class="card">
            <div class="card-body">
                // Selection checkbox and title
                <div class="d-flex align-items-start mb-2">
                    <div class="form-check me-2">
                        <input
                            class="form-check-input"
                            type="checkbox"
                            checked={is_selected}
                            onchange={onchange}
                        />
                    </div>
                    <h5 class="card-title mb-0 flex-grow-1">
                        {article.title.clone().unwrap_or_default()}
                    </h5>
                </div>

                // Metadata
                <div class="mb-2">
                    {if let Some(authors) = &article.first_author {
                        html! { <div class="text-muted"><i class="bi bi-person"></i> {" "}{authors}</div> }
                    } else {
                        html! {}
                    }}

                    {if let Some(journal) = &article.journal {
                        html! { <div class="text-muted"><i class="bi bi-journal"></i> {" "}{journal}</div> }
                    } else {
                        html! {}
                    }}

                    <div class="text-muted">
                        {if let Some(year) = &article.year_published {
                            html! { <span><i class="bi bi-calendar"></i> {" "}{year}</span> }
                        } else {
                            html! {}
                        }}

                        {if let Some(citations) = article.citations {
                            html! { <span class="ms-3"><i class="bi bi-quote"></i> {" "}{citations}{" citations"}</span> }
                        } else {
                            html! {}
                        }}

                        {if let Some(score) = article.score {
                            html! { <span class="ms-3"><i class="bi bi-star"></i> {" Score: "}{score}</span> }
                        } else {
                            html! {}
                        }}
                    </div>
                </div>

                // Summary with collapse/expand
                {if let Some(summary) = &article.summary {
                    html! {
                        <div class="mb-2">
                            <p class={classes!("card-text", "mb-1", if !*summary_expanded { "text-truncate" } else { "" })}>
                                {summary}
                            </p>
                            <a href="#" onclick={toggle_summary} class="text-decoration-none small">
                                {if *summary_expanded { "Show less" } else { "Show more" }}
                            </a>
                        </div>
                    }
                } else {
                    html! {}
                }}

                // DOI link
                {if let Some(link) = doi_link {
                    html! {
                        <a href={link} target="_blank" class="btn btn-sm btn-outline-primary">
                            <i class="bi bi-link-45deg"></i> {" View Article"}
                        </a>
                    }
                } else {
                    html! {}
                }}
            </div>
        </div>
    }
}
