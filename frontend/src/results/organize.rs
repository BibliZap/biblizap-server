use yew::prelude::*;

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
