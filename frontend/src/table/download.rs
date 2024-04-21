use web_sys::HtmlElement;
use wasm_bindgen::prelude::*;
use yew::prelude::*;

use crate::common;
use crate::table::Article;

#[allow(dead_code)]
pub fn to_csv(articles: &[Article]) -> Result<Vec<u8>, common::Error> {
    let mut wtr = csv::Writer::from_writer(Vec::new());

    for article in articles.iter() {
        wtr.serialize(article)?;
    }

    wtr.flush()?;

    match wtr.into_inner() {
        Ok(vec) => Ok(vec),
        Err(error) => Err(common::Error::CsvIntoInner(error.to_string()))
    }
}

pub fn to_excel(articles: &[Article]) -> Result<Vec<u8>, common::Error> {
    use rust_xlsxwriter::Workbook;

    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    worksheet.write_string(0, 0, "doi")?;
    worksheet.write_string(0, 1, "Title")?;
    worksheet.write_string(0, 2, "Journal")?;
    worksheet.write_string(0, 3, "Year published")?;
    worksheet.write_string(0, 4, "Summary")?;
    worksheet.write_string(0, 5, "Citations")?;
    worksheet.write_string(0, 6, "Score")?;

    let text_format = rust_xlsxwriter::Format::new()
        .set_text_wrap()
        .set_align(rust_xlsxwriter::FormatAlign::Top);

    for col in 0..=6 {
        worksheet.set_column_format(col, &text_format)?;
    }
    

    for (i, article) in articles.iter().enumerate() {
        let i : u32 = i.try_into()?;

        worksheet.write_string(i + 1, 0, article.doi.clone().unwrap_or_default())?;
        worksheet.write_string(i + 1, 1, article.title.clone().unwrap_or_default())?;
        worksheet.write_string(i + 1, 2, article.journal.clone().unwrap_or_default())?;
        worksheet.write_string(i + 1, 3, article.year_published.unwrap_or_default().to_string())?;
        worksheet.write_string(i + 1, 4, article.summary.clone().unwrap_or_default())?;
        worksheet.write_string(i + 1, 5, article.citations.unwrap_or_default().to_string())?;
        worksheet.write_string(i + 1, 6, article.score.unwrap_or_default().to_string())?;

        worksheet.set_row_height(i + 1, 150)?;
    }

    worksheet.autofit();
    worksheet.set_column_width(1, 52)?;
    worksheet.set_column_width(2, 52)?;
    worksheet.set_column_width(4, 52)?;
    worksheet.autofilter(0, 0, articles.len().try_into()?, 6)?;
    

    let buf = workbook.save_to_buffer()?;

    Ok(buf)
}

pub fn download_bytes_as_file(bytes: &[u8], filename: &str) -> Result<(), common::Error> {
    use gloo_utils::document;
    let file_blob= gloo_file::Blob::new(bytes);
    let download_url = web_sys::Url::create_object_url_with_blob(&file_blob.into())?;

    let a = document()
        .create_element("a")?;
    
    a.set_attribute("href", &download_url)?;
    a.set_attribute("download", filename)?;
    a.dyn_ref::<HtmlElement>().ok_or(common::Error::HtmlElementDynRef)?.click();

    document().remove_child(&a)?;

    Ok(())
}


#[derive(Clone, PartialEq, Properties)]
pub struct ButtonProps {
    pub onclick: Callback<MouseEvent>
}

#[function_component(DownloadButton)]
pub fn download_button(props: &ButtonProps) -> Html {
    html! {
        <div>
            <button class="btn btn-outline-secondary btn-lg mb-10" onclick={props.onclick.clone()}><i class="bi bi-download me-2"></i>{"Download articles"}</button>
        </div>
    }
}