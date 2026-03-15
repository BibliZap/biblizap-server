use std::ops::Deref;

use yew::prelude::*;
use yew_router::prelude::*;

use crate::common::Route;

/// Properties for the NavBar component.
#[derive(Clone, PartialEq, Properties)]
pub struct NavBarProps {
    pub dark_mode: UseStateHandle<bool>,
}

/// Navigation bar component.
/// Allows switching between different pages and toggling dark mode.
#[function_component]
pub fn NavBar(props: &NavBarProps) -> Html {
    let route = use_route::<Route>();

    let toggle_dark_mode = {
        let dark_mode = props.dark_mode.clone();
        Callback::from(move |_: MouseEvent| {
            dark_mode.set(!dark_mode.deref());
        })
    };

    html! {
    <nav class="navbar navbar-expand-lg bg-body-tertiary">
        <div class="container-fluid">
            <Link<Route> to={Route::BibliZapSearch} classes="navbar-brand">
                <img src="/icons/biblizap-nosnowball-round-fill.svg" alt="" width="50" height="50" class="px-2"/>
                {"BibliZap"}
            </Link<Route>>
            <button class="navbar-toggler" type="button" data-bs-toggle="collapse" data-bs-target="#navbarSupportedContent" aria-controls="navbarSupportedContent" aria-expanded="false" aria-label="Toggle navigation">
                <span class="navbar-toggler-icon"></span>
            </button>
            <div id="navbarSupportedContent" class="collapse navbar-collapse">
                <ul class="navbar-nav navbar-expand-lg">
                    <li class="nav-item">
                        <Link<Route> to={Route::BibliZapSearch}
                            classes={if route == Some(Route::BibliZapSearch) { "nav-link active" } else { "nav-link" }}>
                            <i class="bi bi-house-fill px-2"></i>
                            {"App"}
                        </Link<Route>>
                    </li>
                    <li class="nav-item">
                        <Link<Route> to={Route::HowItWorks}
                            classes={if route == Some(Route::HowItWorks) { "nav-link active" } else { "nav-link" }}>
                            <i class="bi bi-lightbulb-fill px-2"></i>
                            {"How it works"}
                        </Link<Route>>
                    </li>
                    <li class="nav-item">
                        <Link<Route> to={Route::Contact}
                            classes={if route == Some(Route::Contact) { "nav-link active" } else { "nav-link" }}>
                            <i class="bi bi-send-fill px-2"></i>
                            {"Contact"}
                        </Link<Route>>
                    </li>
                    <li class="nav-item">
                        <Link<Route> to={Route::LegalInformation}
                            classes={if route == Some(Route::LegalInformation) { "nav-link active" } else { "nav-link" }}>
                            <i class="bi bi-info-circle-fill px-2"></i>
                            {"Legal information"}
                        </Link<Route>>
                    </li>
                    <BrowserPluginNavItem/>
                    <li class="nav-item" onclick={toggle_dark_mode}>
                        <button class="nav-link active">
                        if *props.dark_mode.deref() {
                            <i class="bi bi-sun-fill px-2"></i>
                        } else {
                            <i class="bi bi-moon-fill px-2"></i>
                        }
                        </button>
                    </li>
                </ul>
            </div>
        </div>
    </nav>
    }
}

/// Component to conditionally display a link to the browser plugin.
/// Checks the user agent to determine if a Firefox plugin link should be shown.
#[function_component]
pub fn BrowserPluginNavItem() -> Html {
    use crate::common::{Error, WebBrowser};
    let window = web_sys::window().expect("Missing Window");

    let browser: Result<WebBrowser, Error> = window.navigator().try_into();

    match browser {
        Ok(browser) => match browser {
            WebBrowser::Firefox => html! { <FirefoxPluginNavItem/> },
            WebBrowser::Chrome => html! {},
        },
        Err(_) => html! {},
    }
}

/// Navigation item specifically for the Firefox browser plugin link.
#[function_component]
pub fn FirefoxPluginNavItem() -> Html {
    html! {
        <li class="nav-item">
            <a class="nav-link" href="https://addons.mozilla.org/firefox/addon/biblizap/">
                <i class="bi bi-browser-firefox px-2"></i>
                {"Firefox Plugin"}
            </a>
        </li>
    }
}
