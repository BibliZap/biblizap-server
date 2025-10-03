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

    html! {
        <tr>
            <td><input type={"checkbox"} class={"row-checkbox"} checked={is_selected} onchange={onchange}/></td>
            <td style=""><a href={doi_link(props.article.doi.clone())} style="word-wrap: break-word">{props.article.doi.clone().unwrap_or_default()}</a></td>
            <td style="word-wrap: break-word">{props.article.title.clone().unwrap_or_default()}</td>
            <td style="word-wrap: break-word">{props.article.journal.clone().unwrap_or_default()}</td>
            <td>{props.article.first_author.clone().unwrap_or_default()}</td>
            <td>{props.article.year_published.unwrap_or_default()}</td>
            <td>{props.article.summary.clone().unwrap_or_default()}</td>
            <td>{props.article.citations.unwrap_or_default()}</td>
            <td>{props.article.score.unwrap_or_default()}</td>
        </tr>
    }
}
