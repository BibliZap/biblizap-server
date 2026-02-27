use rand::seq::SliceRandom;
use yew::prelude::*;

/// Component for displaying legal information, including disclaimer, privacy policy, etc.
#[function_component(LegalInformation)]
pub fn legal_information() -> Html {
    let mut rng = rand::thread_rng();

    let mut creators: [&str; 3] = ["Bastien Le Guellec", "Raphaël Bentégeac", "Victor Leblanc"];
    creators.shuffle(&mut rng);

    let creators = creators.join(", ");

    html! {
        <div class="container-md">
            <h1 class="mb-4"><i class="bi bi-info-circle-fill px-2"></i>{"Legal Information"}</h1>
            <h3>{"Disclaimer"}</h3>

            <p class="p-3">
                {"The information provided on this website is for general informational purposes only and is extracted from the website Lens.org."}<br/>
                {"While we strive to keep the information up to date and accurate, we make no representations or warranties of any kind, express or implied, about the completeness, accuracy, reliability, suitability, or availability with respect to the website or the information, products, services, or related graphics contained on the website for any purpose."}<br/>
                {"Any reliance you place on such information is therefore strictly at your own risk."}
            </p>
            <h3>{"Privacy Policy"}</h3>
            <p class="p-3">
                {"No personal information is collected on this website."}
            </p>
            <h3>{"Intellectual Property"}</h3>
            <p class="p-3">
                {"All intellectual property rights in and to the content and materials on this website are owned by us or our licensors."}<br/>
                {"You may not use, reproduce, distribute, or otherwise exploit any content from this website without our prior written consent."}
            </p>

            <h3>{"Web host"}</h3>
            <p class="p-3">
                {"Website hosted by OVH"}<br/>
                {"2 rue Kellermann"}<br/>
                {"BP 80157 59053 ROUBAIX CEDEX 1"}<br/>
                {"FRANCE"}
            </p>

            <h3>{"Inventors"}</h3>
            <p class="p-3">
                {"BibliZap was created and developped by :"}<br/>
                {creators}
            </p>

        </div>
    }
}

/// Component for explaining how BibliZap works, its principles, and data sources.
#[function_component(HowItWorks)]
pub fn how_it_works() -> Html {
    html! {
        <div class="container-md">
            <h1 class="mb-4"><i class="bi bi-lightbulb-fill px-2"></i>{"General principle"}</h1>
            <h3>{"BibliZap is a free and open-source project"}</h3>

            <p class="p-3">
                {"BibliZap aims to catalog articles similar to the source article based on bidirectional citation searching."}<br/>
                {"Downward citations correspond to the references of the articles (their bibliography)."}<br/>
                {"Upward citations correspond to the articles citing the source article."}
            </p>
            <h3>{"Here is a diagram summarizing the process:"}</h3>
            <div class="container">
                <div class="row">
                    <div class="col-md">
                        <img src="icons/BibliZapFig1.1.svg" class="p-3 img-fluid"/>
                    </div>
                    <div class="col-md">
                        <img src="icons/BibliZapFig1.1.svg" class="p-3 img-fluid"/>
                    </div>
                </div>
            </div>



            <p class="p-3">{"At each level, the number of times each PMID appears is recorded. At the end of the process, the sum of occurrences provides the score. For instance, if an article is found once in the references of the source article, then is discovered 6 times in the articles cited by the articles that are cited by the source article, and is not found elsewhere, its score will be 7."}</p>
            <h1 class="mb-4"><i class="bi bi-database-fill px-2"></i>{"Data sources"}</h1>
            <p class="p-3">{"Meta-data from articles are provided by The Lens, a not-for-profit service from Cambia. The Lens gathers and harmonises bibliographic data from different sources (Crossref, PubMed, Microsoft Academic, ...)"}</p>


            <div class="container">
                <div class="row">
                    <div class="col-md">
                        <img src="icons/scholar-venn.png" class="p-3 img-fluid"/>
                    </div>
                    <div class="col-md">
                        <img src="icons/scholar-chart.png" class="p-3 img-fluid"/>
                    </div>
                </div>
            </div>
            <p class="p-3">
                {"Using the BibliZap web-app freely is possible thanks to The Lens generously providing an API access to all users of the BibliZap web-app."}<br/>
                {"Users of the R package will need a spectific individual token which can be obtained through The Lens for 14 days."}<br/>
                {"BibliZap does not receive financial support from The Lens or Cambia, or any other enterprise or journal."}
            </p>
            <h1><i class="bi bi-graph-down-arrow px-2"></i>{"Is there a risk that BibliZap might contribute to citation bias ?"}</h1>
            <p class="p-3">
                {"Yes, there is a potential risk of BibliZap contributing to citation bias."}<br/>
                {"Therefore, it is extremely important to always conduct keyword-based article searches in parallel."}<br/>
                {"This is especially crucial if you intend to publish your work."}
            </p>
        </div>
    }
}

/// Component for displaying contact information.
#[function_component(Contact)]
pub fn contact() -> Html {
    html! {
        <div class="container-md">
            <h1 class="mb-4"><i class="bi bi-send-fill px-2"></i>{"Contact"}</h1>
            <h3>{"Issues"}</h3>

            <p class="p-3">
                {"Regarding issues you may go to "}<a href={"https://github.com/BibliZap/BibliZap"}>{"our github repo"}</a><br/>
                {"Don't forget to search the existing issues for something similar."}<br/>
                {"You may also ask for new features in that manner."}
            </p>

            <h3>{"Contact"}</h3>
            <p class="p-3">
                {"If you want to send us a message you may use our mail adress :"}<br/>
                <a href={"mailto:BibliZap Contact <contact@biblizap.org>"}>{"contact@biblizap.org"}</a>
            </p>

        </div>
    }
}
