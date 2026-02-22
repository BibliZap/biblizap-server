use yew::prelude::*;

/// Component for the footer section.
#[function_component]
pub fn Wall() -> Html {
    html! {
        <footer class="footer mt-auto py-3 bg-body-tertiary">
            <div class="container">
                <div class="row">
                    <div class="col-md-6 text-center text-md-start mb-2 mb-md-0">
                        <img src="/icons/biblizap-snowball-round-fill.svg" alt="BibliZap" width="30" height="30" class="me-2" style="vertical-align: middle;"/>
                        <strong>{"BibliZap"}</strong>
                        <span class="text-muted ms-2">{"Citation searching made easy"}</span>
                    </div>
                    <div class="col-md-6 text-center text-md-end">
                        <span class="text-muted">{"Powered by "}<a href="https://www.lens.org/" class="text-decoration-none">{"the Lens"}</a></span>
                        <span class="text-muted ms-3">{"Trusted by the Faculty of Medicine of Lille"}</span>
                    </div>
                </div>
            </div>
        </footer>
    }
}
