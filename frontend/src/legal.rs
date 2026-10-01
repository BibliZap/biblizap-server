use rand::seq::SliceRandom;
use yew::prelude::*;

/// Component for displaying legal information, including disclaimer, privacy policy, etc.
#[function_component(LegalInformation)]
pub fn legal_information() -> Html {
    let mut rng = rand::rng();

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

const METHODS_REPORTING_TEMPLATE: &str = concat!(
    "To supplement the primary [PubMed / Embase / other] database search, we performed citation searching using BibliZap (https://biblizap.org), an open-source automated citation searching tool. ",
    "Citation searching was conducted on [date], using as seed references [all records meeting inclusion criteria after full-text screening of the primary search / a purposive sample of [n] records drawn from those included after full-text screening, selected on the basis of [rationale]] ",
    "(n = [X] seed references; identifiers provided in Supplementary Appendix [X]). ",
    "The search was performed to a depth of [one / two] citation levels in [both directions / citations only / references only]. ",
    "BibliZap ranked records by citation-path score, using ascending Lens ID as a deterministic tie-breaker. ",
    "A RIS file of records identified through the primary database search was uploaded as an exclusion list; records with matching DOIs were hidden from the displayed results after ranking. ",
    "The output limit was set to the top [100 / 500 / 1000 / all] ranked records before exclusion; [X] displayed records were screened by title and abstract."
);

const RESULTS_REPORTING_TEMPLATE: &str = concat!(
    "Citation searching with BibliZap displayed [X] candidate records after DOI-based exclusion of records already identified through the primary search. ",
    "Of these, [X] were screened by title and abstract; [X] proceeded to full-text review, and [X] were ultimately included. ",
    "Reasons for full-text exclusion and the results are reflected in the right-hand column of the PRISMA 2020 flow diagram (Figure [X])."
);

/// Component for explaining how BibliZap works, its principles, and data sources.
#[function_component(HowItWorks)]
pub fn how_it_works() -> Html {
    use_effect_with((), |_| {
        // The section is rendered by Yew after the browser's initial fragment jump.
        if let Some(window) = web_sys::window() {
            if window.location().hash().ok().as_deref() == Some("#how-to-report-a-biblizap-search")
            {
                if let Some(section) = window.document().and_then(|document| {
                    document.get_element_by_id("how-to-report-a-biblizap-search")
                }) {
                    section.scroll_into_view();
                }
            }
        }
        || ()
    });

    html! {
        <div class="container-md">
            <h1 class="mb-4"><i class="bi bi-lightbulb-fill px-2"></i>{"General principle"}</h1>
            <h3>{"BibliZap is a free and open-source project"}</h3>

            <p class="p-3">
                {"BibliZap aims to catalog articles similar to the source article based on bidirectional citation searching."}<br/>
                {"Downward citations correspond to the references of the articles (their bibliography)."}<br/>
                {"Upward citations correspond to the articles citing the source article."}
            </p>
            <section class="mb-4" aria-labelledby="published-evaluation">
                <h2 id="published-evaluation" class="h3">{"Published evaluation"}</h2>
                <p class="p-3">
                    {"Bentegeac R, Le Guellec B, Leblanc V, et al. BibliZap: An Exploratory Evaluation of an Automated Multi-Level Citation Searching Tool for Systematic and Rapid Reviews. "}
                    <cite>{"Research Synthesis Methods"}</cite>{". 2026;17(4):816–829. "}
                    <a href="https://doi.org/10.1017/rsm.2026.10079">{"https://doi.org/10.1017/rsm.2026.10079"}</a>
                </p>
            </section>
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



            <p class="p-3">{"At each level, BibliZap counts citation paths reaching each article, identified internally by Lens ID. The sum across levels is its citation-path score. Records are ranked by descending score; ascending Lens ID resolves ties deterministically."}</p>
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
            <section id="how-to-report-a-biblizap-search" class="mb-5" aria-labelledby="reporting-heading">
                <h2 id="reporting-heading">{"How to report a BibliZap search"}</h2>
                <p>
                    {"For reproducibility, report the search date; seed identifiers (up to 100 in the web app, ideally listed in a supplement) and the rationale for any sampled seeds; Depth (one or two levels) and Search direction (Both, Citations, or References); any DOI-based exclusion file; the Number of results setting (100, 500, 1000, or All); and the numbers displayed, screened, assessed at full text, and included."}
                </p>
                <p class="alert alert-info">
                    {"BibliZap ranks and applies the result limit before DOI-based exclusions are hidden from the displayed results. An uploaded RIS exclusion file is read for DOIs, so records without a matching DOI are not excluded. A top-500 limit can therefore display fewer than 500 records after exclusion."}
                </p>
                <p>
                    {"The bracketed alternatives and values below are examples to replace with what you actually used. Adapt or delete sentences about options you did not use (including exclusion files), and adjust the PRISMA flow-diagram sentence to your own diagram."}
                </p>
                <h3 class="h4">{"Methods — copy-ready example"}</h3>
                <div class="border rounded bg-body-tertiary p-3 mb-4">
                    <p class="mb-0">{METHODS_REPORTING_TEMPLATE}</p>
                </div>
                <h3 class="h4">{"Results — copy-ready example"}</h3>
                <div class="border rounded bg-body-tertiary p-3">
                    <p class="mb-0">{RESULTS_REPORTING_TEMPLATE}</p>
                </div>
            </section>
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
