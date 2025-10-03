use yew::prelude::*;

/// Component for the main title and logo section ("the wall").
#[function_component]
pub fn Wall() -> Html {
    html! {
        <div class="container text-center my-5">
            <h1 class="main-title">
                <img src="/icons/biblizap-snowball-round-fill.svg" id="logo" alt="" width="300vw" style="margin-bottom: 50px"/>
                {"BibliZap"}
            </h1>
            <h5 class="text-end">{"Citation searching made easy"}</h5>
            <h5 class="text-end">{"Powered by "}<a href="https://www.lens.org/">{"the Lens"}</a></h5>
            <h5 class="text-end">{"Trusted by the Faculty of Medecine of Lille"}</h5>
        </div>
    }
}
