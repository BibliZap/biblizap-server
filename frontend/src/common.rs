use thiserror::Error;
use wasm_bindgen::JsValue;
use web_sys::Navigator;
use yew::prelude::*;
use yew_router::prelude::*;

/// Custom error type for the frontend application.
#[derive(Error, Debug)]
pub enum Error {
    #[error("Api error: {0}")]
    Api(String),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Request(#[from] gloo_net::Error),
    #[error(transparent)]
    Csv(#[from] csv::Error),
    #[error(transparent)]
    Xlsx(#[from] rust_xlsxwriter::XlsxError),
    #[error("Csv into_inner error")]
    CsvIntoInner(String),
    #[error("JsValue error")]
    JsValueString(String),
    #[error(transparent)]
    TryFromInt(#[from] std::num::TryFromIntError),
    #[error(transparent)]
    ParseInt(#[from] std::num::ParseIntError),
    #[error("HtmlElement dyn_ref error")]
    HtmlElementDynRef,
    #[error(transparent)]
    NodeRefMissingValue(#[from] NodeRefMissingValue),
    #[error(transparent)]
    UrlParse(#[from] url::ParseError),
    #[error("Unrecognized User Agent : {0}")]
    UnrecognizedUserAgent(String),
    #[error("Invalid identifier format: '{0}' is neither a valid DOI nor PMID")]
    InvalidIdFormat(String),
    #[error("Too many identifiers: maximum 10 allowed, got {0}")]
    TooManyIds(usize),
    #[error("No valid identifiers provided")]
    NoValidIds,
}

/// Enum representing missing values from NodeRefs.
#[derive(Error, Debug)]
pub enum NodeRefMissingValue {
    #[error("Id list is missing")]
    IdList,
    #[error("Output max size is missing")]
    OutputMaxSize,
    #[error("Depth is missing")]
    Depth,
    #[error("SearchFor is missing")]
    SearchFor,
}

impl From<JsValue> for Error {
    fn from(value: JsValue) -> Self {
        Error::JsValueString(value.as_string().unwrap_or_default())
    }
}

/// Query params for `/biblizap-results?ids=…`
#[derive(Clone, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
pub struct BibliZapResultsQuery {
    /// Space-separated list of DOIs / PMIDs.
    pub ids: String,
    /// Snowball depth (1 or 2). Defaults to 2 when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub depth: Option<u8>,
    /// Max number of output results. Defaults to `Limit(100)` when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_max_size: Option<OutputMaxSize>,
    /// Search direction. Defaults to `Both` when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub search_for: Option<SearchFor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub denylist_hash: Option<String>,
}

/// Prop for `SnowballForm` indicating the form's current position.
/// Controls whether the *next* page's form will animate into position.
/// `Centered` → form is on the landing page; next page will animate.
/// `Top` → form is already at top (results page); next page won't animate.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum FormPosition {
    Centered,
    TopAnimated,
    #[default]
    Top,
}

impl FormPosition {
    pub fn next(&self) -> Self {
        match self {
            FormPosition::Centered => FormPosition::TopAnimated,
            FormPosition::TopAnimated => FormPosition::Top,
            FormPosition::Top => FormPosition::Top,
        }
    }

    pub fn get_class(&self) -> &'static str {
        match self {
            FormPosition::Centered => "form-container-centered",
            FormPosition::TopAnimated => "form-container-top-animated",
            FormPosition::Top => "form-container-top",
        }
    }
}

#[derive(Clone, Routable, PartialEq)]
pub enum Route {
    #[at("/")]
    BibliZapSearch,
    #[at("/how-it-works")]
    HowItWorks,
    #[at("/pubmed-results")]
    PubMedResults,
    #[at("/biblizap-results")]
    BibliZapResults,
    #[at("/contact")]
    Contact,
    #[at("/legal")]
    LegalInformation,
    #[not_found]
    #[at("/404")]
    NotFound,
}

/// Enum representing the direction of the snowball search.
#[derive(Clone, Copy, PartialEq, Default, Debug, serde::Serialize, serde::Deserialize)]
pub enum SearchFor {
    References,
    Citations,
    #[default]
    Both,
}

/// Enum representing the maximum number of output results.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum OutputMaxSize {
    Limit(usize),
    All,
}

impl Default for OutputMaxSize {
    fn default() -> Self {
        OutputMaxSize::Limit(100)
    }
}

impl serde::Serialize for OutputMaxSize {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            OutputMaxSize::All => serializer.serialize_str("All"),
            OutputMaxSize::Limit(n) => serializer.serialize_str(&n.to_string()),
        }
    }
}

impl<'de> serde::Deserialize<'de> for OutputMaxSize {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "All" => Ok(OutputMaxSize::All),
            n => n
                .parse::<usize>()
                .map(OutputMaxSize::Limit)
                .map_err(serde::de::Error::custom),
        }
    }
}

/// Helper function to get the value from an HTML input element referenced by a NodeRef.
pub fn get_value(node_ref: &NodeRef) -> Option<String> {
    Some(node_ref.cast::<web_sys::HtmlInputElement>()?.value())
}

pub enum WebBrowser {
    Firefox,
    Chrome,
}

impl TryFrom<Navigator> for WebBrowser {
    type Error = Error;

    /// Attempts to determine the web browser from the Navigator object.
    fn try_from(navigator: Navigator) -> Result<Self, Self::Error> {
        let user_agent: String = navigator.user_agent()?;

        if user_agent.contains("Firefox") {
            return Ok(Self::Firefox);
        } else if user_agent.contains("Chrome") {
            return Ok(Self::Chrome);
        }

        Err(Error::UnrecognizedUserAgent(user_agent))
    }
}
