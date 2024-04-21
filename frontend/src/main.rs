use std::{ops::Deref, cell::RefCell};
use std::rc::Rc;

use yew::prelude::*;

mod legal;
use legal::*;

mod table;
use table::*;

mod navbar;
use navbar::*;

mod wall;
use wall::*;

mod form;
use form::SnowballForm;

mod common;
use common::{Error, CurrentPage};

#[function_component(App)]
fn app() -> Html {
    let current_page = use_state(|| CurrentPage::BibliZapApp);
    let dark_mode = use_state(|| false);
    match dark_mode.deref() {
        true => gloo_utils::document_element().set_attribute("data-bs-theme", "dark").unwrap_or(()),
        false => gloo_utils::document_element().set_attribute("data-bs-theme", "light").unwrap_or(())
    }
    
    let content = match current_page.deref() {
        CurrentPage::BibliZapApp => { html!{<BibliZapApp/>} },
        CurrentPage::HowItWorks => { html!{<HowItWorks/>} },
        CurrentPage::LegalInformation => { html!{<LegalInformation/>} },
        CurrentPage::Contact => { html!{<Contact/>} }
    };
    html! {
        <div>
            <NavBar current_page={current_page} dark_mode={dark_mode}/>
            <Wall/>
            {content}
        </div>
    }
}   

#[function_component(BibliZapApp)]
fn app() -> Html {
    let table_status = use_state(|| TableStatus::NotRequested);
    let on_receiving_response = { 
        let table_status = table_status.clone();
        Callback::from(move |table: Result<Rc<RefCell<Vec<Article>>>, Error>| {
            match table {
                Ok(table) => table_status.set(TableStatus::Available(table)),
                Err(error) => table_status.set(TableStatus::RequestError(error.to_string())),
            };
        })
    };
    let on_requesting_table = {
        let table_status = table_status.clone();
        Callback::from(move |_: ()| {
            table_status.set(TableStatus::Requested);
        })
    };

    let on_submit_error= {
        let table_status = table_status.clone();
        Callback::from(move |error: common::Error| {
            table_status.set(TableStatus::RequestError(error.to_string()))
        })
    };

    html! {
        <div>
            <SnowballForm {on_submit_error} {on_requesting_table} {on_receiving_response}/>
            <TableContainer table_status={table_status.clone()}/>
        </div>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}