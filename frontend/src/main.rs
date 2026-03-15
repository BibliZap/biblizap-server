use std::ops::Deref;

use yew::prelude::*;

mod legal;
use legal::*;

mod results;
use results::*;

mod navbar;
use navbar::*;

mod wall;
use wall::*;

mod search;
use search::*;

mod common;

pub mod pubmed;

use yew_router::prelude::*;

use crate::common::Route;

/// The main application component.
/// Manages the current page state and dark mode state.
#[function_component(App)]
fn app() -> Html {
    let dark_mode = use_state(|| false);

    match dark_mode.deref() {
        true => gloo_utils::document_element()
            .set_attribute("data-bs-theme", "dark")
            .unwrap_or(()),
        false => gloo_utils::document_element()
            .set_attribute("data-bs-theme", "light")
            .unwrap_or(()),
    }

    html! {
        <BrowserRouter>
        <div class="d-flex flex-column min-vh-100">
            <NavBar dark_mode={dark_mode}/>
            <div class="container my-4">
                    <Switch<Route> render={switch} />
            </div>
            <Wall/>
        </div>
        </BrowserRouter>
    }
}

fn switch(routes: Route) -> Html {
    match routes {
        Route::BibliZapSearch => html! { <BibliZapSearchPage/> },
        Route::PubMedResults => html! { "not implemented" },
        Route::BibliZapResults => html! { <BibliZapResults /> },
        Route::HowItWorks => html! { <HowItWorks /> },
        Route::Contact => html! { <Contact /> },
        Route::LegalInformation => html! { <LegalInformation /> },
        Route::NotFound => html! { <BibliZapSearchPage/> },
    }
}

/// Entry point for the Yew frontend application.
fn main() {
    yew::Renderer::<App>::new().render();
}
