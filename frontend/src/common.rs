use thiserror::Error;
use wasm_bindgen::JsValue;
use web_sys::Navigator;
use yew::prelude::*;

#[derive(Error, Debug)]
pub enum Error {
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
    UnrecognizedUserAgent(String)
}

#[derive(Error, Debug)]
pub enum NodeRefMissingValue {
    #[error("Id list is missing")]
    IdList,
    #[error("Output max size is missing")]
    OutputMaxSize,
    #[error("Depth is missing")]
    Depth,
    #[error("SearchFor is missing")]
    SearchFor
}

impl From<JsValue> for Error {
    fn from(value: JsValue) -> Self {
        Error::JsValueString(value.as_string().unwrap_or_default())
    }
}

#[derive(PartialEq)]
pub enum CurrentPage {
    BibliZapApp,
    HowItWorks,
    Contact,
    LegalInformation
}

#[derive(Clone, PartialEq, Default, Debug, serde::Serialize)]
pub enum SearchFor {
    References,
    Citations,
    #[default]
    Both
}

pub fn get_value(node_ref: &NodeRef) -> Option<String> {
    Some(node_ref.cast::<web_sys::HtmlInputElement>()?.value())
}


pub enum WebBrowser {
    Firefox,
    Chrome
}

impl TryFrom<Navigator> for WebBrowser {
    type Error = Error;

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