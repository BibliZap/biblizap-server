use std::collections::HashSet;
use yew::prelude::*;

use crate::common::PubmedSearchResult;

/// Properties for the PubMedResults component.
#[derive(Clone, PartialEq, Properties)]
pub struct PubmedResultsProps {
    /// The list of PubMed search results to display.
    pub results: Vec<PubmedSearchResult>,
    /// Callback when the user clicks "Run BibliZap with selected articles".
    /// Passes a list of identifiers (DOI preferred, PMID as fallback).
    pub on_run_snowball: Callback<Vec<String>>,
}

/// Component for displaying PubMed keyword search results with selection checkboxes.
/// Users can select articles and then run BibliZap snowball on the selected ones.
#[function_component(PubMedResultsView)]
pub fn pubmed_results_view(props: &PubmedResultsProps) -> Html {
    let selected = use_state(|| HashSet::<String>::new());

    let toggle_all = {
        let selected = selected.clone();
        let results = props.results.clone();
        Callback::from(move |_: MouseEvent| {
            let mut current = (*selected).clone();
            if current.len() == results.len() {
                current.clear();
            } else {
                current = results.iter().map(|r| r.pmid.clone()).collect();
            }
            selected.set(current);
        })
    };

    let on_run = {
        let selected = selected.clone();
        let results = props.results.clone();
        let on_run_snowball = props.on_run_snowball.clone();
        Callback::from(move |_: MouseEvent| {
            // For each selected PMID, prefer DOI if available (Lens.org has better DOI coverage)
            let ids: Vec<String> = results.iter()
                .filter(|r| selected.contains(&r.pmid))
                .map(|r| {
                    r.doi.clone().unwrap_or_else(|| r.pmid.clone())
                })
                .collect();
            on_run_snowball.emit(ids);
        })
    };

    let all_selected = selected.len() == props.results.len() && !props.results.is_empty();
    let has_selection = !selected.is_empty();

    html! {
        <div class="container-fluid">
            <hr/>
            <div class="d-flex justify-content-between align-items-center mb-3">
                <h5 class="mb-0">
                    {format!("PubMed search returned {} results", props.results.len())}
                </h5>
                <div class="d-flex gap-2 align-items-center">
                    <button
                        type="button"
                        class="btn btn-outline-secondary btn-sm"
                        onclick={toggle_all}
                    >
                        { if all_selected { "Deselect all" } else { "Select all" } }
                    </button>
                    <button
                        type="button"
                        class={classes!(
                            "btn", "btn-primary",
                            if !has_selection { Some("disabled") } else { None }
                        )}
                        onclick={on_run.clone()}
                        disabled={!has_selection}
                    >
                        <i class="bi bi-search"></i>
                        { format!(" Run BibliZap with {} selected", selected.len()) }
                    </button>
                </div>
            </div>

            <div class="table-responsive">
                <table class="table table-hover table-bordered">
                    <thead>
                        <tr>
                            <th style="width:3%"></th>
                            <th>{"PMID"}</th>
                            <th style="width:35%">{"Title"}</th>
                            <th>{"Authors"}</th>
                            <th>{"Journal"}</th>
                            <th>{"Year"}</th>
                        </tr>
                    </thead>
                    <tbody class="table-group-divider">
                        { props.results.iter().map(|article| {
                            let pmid = article.pmid.clone();
                            let is_selected = selected.contains(&pmid);
                            let toggle = {
                                let selected = selected.clone();
                                let pmid = pmid.clone();
                                Callback::from(move |_: MouseEvent| {
                                    let mut current = (*selected).clone();
                                    if current.contains(&pmid) {
                                        current.remove(&pmid);
                                    } else {
                                        current.insert(pmid.clone());
                                    }
                                    selected.set(current);
                                })
                            };

                            let row_class = if is_selected { "table-primary" } else { "" };

                            html! {
                                <tr class={row_class} style="cursor:pointer" onclick={toggle.clone()}>
                                    <td class="text-center">
                                        <input
                                            type="checkbox"
                                            class="form-check-input"
                                            checked={is_selected}
                                            onclick={toggle}
                                        />
                                    </td>
                                    <td>
                                        <a href={format!("https://pubmed.ncbi.nlm.nih.gov/{}/", article.pmid)}
                                           target="_blank"
                                           onclick={Callback::from(|e: MouseEvent| e.stop_propagation())}
                                        >
                                            {&article.pmid}
                                        </a>
                                    </td>
                                    <td>{article.title.as_deref().unwrap_or("—")}</td>
                                    <td>
                                        <small>{article.authors.as_deref().unwrap_or("—")}</small>
                                    </td>
                                    <td>
                                        <small><em>{article.journal.as_deref().unwrap_or("—")}</em></small>
                                    </td>
                                    <td>{article.year.as_deref().unwrap_or("—")}</td>
                                </tr>
                            }
                        }).collect::<Html>() }
                    </tbody>
                </table>
            </div>

            if has_selection {
                <div class="d-flex justify-content-center mt-3 mb-3">
                    <button
                        type="button"
                        class="btn btn-primary btn-lg"
                        onclick={on_run.clone()}
                    >
                        <i class="bi bi-search"></i>
                        { format!(" Run BibliZap with {} selected article{}", selected.len(), if selected.len() > 1 { "s" } else { "" }) }
                    </button>
                </div>
            }
        </div>
    }
}
