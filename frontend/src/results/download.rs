use std::cell::RefCell;
use std::collections::HashSet;
use std::ops::Deref;
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use web_sys::HtmlElement;
use yew::prelude::*;

use crate::common;
use crate::results::Article;

/// Converts a slice of `Article` structs into a CSV byte vector.
#[allow(dead_code)]
pub fn to_csv(articles: &[Article]) -> Result<Vec<u8>, common::Error> {
    let mut wtr = csv::Writer::from_writer(Vec::new());

    for article in articles.iter() {
        wtr.serialize(article)?;
    }

    wtr.flush()?;

    match wtr.into_inner() {
        Ok(vec) => Ok(vec),
        Err(error) => Err(common::Error::CsvIntoInner(error.to_string())),
    }
}

/// Converts a slice of `Article` structs into an Excel (XLSX) byte vector.
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
        let i: u32 = i.try_into()?;

        worksheet.write_string(i + 1, 0, article.doi.clone().unwrap_or_default())?;
        worksheet.write_string(i + 1, 1, article.title.clone().unwrap_or_default())?;
        worksheet.write_string(i + 1, 2, article.journal.clone().unwrap_or_default())?;
        worksheet.write_string(
            i + 1,
            3,
            article.year_published.unwrap_or_default().to_string(),
        )?;
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

/// Converts a slice of `Article` structs into an RIS (Research Information Systems) byte vector.
pub fn to_ris(articles: &[Article]) -> Result<Vec<u8>, common::Error> {
    use std::io::Write;
    let mut ris = Vec::new();

    for article in articles.iter() {
        writeln!(ris, "TY  - JOUR")?;
        if let Some(author) = &article.first_author {
            writeln!(ris, "AU  - {}", author)?;
        }
        if let Some(title) = &article.title {
            writeln!(ris, "TI  - {}", title)?;
        }
        if let Some(journal) = &article.journal {
            writeln!(ris, "JO  - {}", journal)?;
        }
        if let Some(year) = article.year_published {
            writeln!(ris, "PY  - {}", year)?;
        }
        if let Some(summary) = &article.summary {
            writeln!(ris, "AB  - {}", summary)?;
        }

        // DOI can have multiple fields, so we fill all of them
        if let Some(doi) = &article.doi {
            writeln!(ris, "DI  - {}", doi)?;
        }
        if let Some(doi) = &article.doi {
            writeln!(ris, "DOI  - {}", doi)?;
        }
        if let Some(doi) = &article.doi {
            writeln!(ris, "DO  - {}", doi)?;
        }
        writeln!(ris, "ER  - ")?;
    }

    Ok(ris)
}

pub fn to_bibtex(articles: &[Article]) -> Result<Vec<u8>, common::Error> {
    use std::io::Write;
    let mut bibtex = Vec::new();

    for article in articles.iter() {
        let citeid = format!(
            "{}{}-{}-{}",
            article.first_author.clone().unwrap_or_default(),
            article.year_published.unwrap_or_default(),
            article
                .journal
                .clone()
                .unwrap_or_default()
                .chars()
                .take(6)
                .collect::<String>(),
            article
                .title
                .clone()
                .unwrap_or_default()
                .chars()
                .take(6)
                .collect::<String>()
        );
        writeln!(bibtex, "@article{{{},", citeid)?;
        if let Some(author) = &article.first_author {
            writeln!(bibtex, "  author = \"{}\",", author)?;
        }
        if let Some(title) = &article.title {
            writeln!(bibtex, "  title = \"{}\",", title)?;
        }
        if let Some(journal) = &article.journal {
            writeln!(bibtex, "  journal = \"{}\",", journal)?;
        }
        if let Some(year) = article.year_published {
            writeln!(bibtex, "  year = {},", year)?;
        }
        if let Some(summary) = &article.summary {
            writeln!(bibtex, "  abstract = \"{}\",", summary)?;
        }
        if let Some(doi) = &article.doi {
            writeln!(bibtex, "  doi = \"{}\"", doi)?;
        }
        writeln!(bibtex, "}},")?;
    }

    Ok(bibtex)
}

/// Triggers a file download in the browser using a byte slice and filename.
pub fn download_bytes_as_file(bytes: &[u8], filename: &str) -> Result<(), common::Error> {
    use gloo_utils::document;
    let file_blob = gloo_file::Blob::new(bytes);
    let download_url = web_sys::Url::create_object_url_with_blob(&file_blob.into())?;

    let a = document().create_element("a")?;

    a.set_attribute("href", &download_url)?;
    a.set_attribute("download", filename)?;
    a.dyn_ref::<HtmlElement>()
        .ok_or(common::Error::HtmlElementDynRef)?
        .click();

    document().remove_child(&a)?;

    Ok(())
}

#[derive(Clone, PartialEq, Properties)]
pub struct ButtonsProps {
    pub articles: Rc<RefCell<Vec<Article>>>,
    pub selected_articles: HashSet<String>,
}

pub fn get_articles_to_download(
    articles: &Rc<RefCell<Vec<Article>>>,
    selected_articles: &HashSet<String>,
) -> Vec<Article> {
    let articles = articles.borrow();

    if selected_articles.is_empty() {
        articles.clone()
    } else {
        articles
            .iter()
            .filter(|article| {
                if let Some(doi) = &article.doi {
                    selected_articles.contains(doi)
                } else {
                    false
                }
            })
            .cloned()
            .collect()
    }
}

#[function_component]
pub fn DownloadButtons(
    ButtonsProps {
        articles,
        selected_articles,
    }: &ButtonsProps,
) -> Html {
    let on_excel_download_click = {
        let articles = articles.clone();
        let selected_articles = selected_articles.clone();
        Callback::from(move |_: MouseEvent| {
            let articles_to_download = get_articles_to_download(&articles, &selected_articles);
            let bytes = to_excel(&articles_to_download).unwrap();
            let timestamp = chrono::Local::now().to_rfc3339();
            let suffix = if articles_to_download.len() == articles.deref().borrow().len() {
                "all"
            } else {
                "selected"
            };

            match download_bytes_as_file(&bytes, &format!("BibliZap-{suffix}-{timestamp}.xlsx")) {
                Ok(_) => (),
                Err(error) => {
                    gloo_console::log!(format!("{error}"));
                }
            }
        })
    };

    let on_ris_download_click = {
        let articles = articles.clone();
        let selected_articles = selected_articles.clone();
        Callback::from(move |_: MouseEvent| {
            let articles_to_download = get_articles_to_download(&articles, &selected_articles);
            let bytes = to_ris(&articles_to_download).unwrap();
            let timestamp = chrono::Local::now().to_rfc3339();
            let suffix = if articles_to_download.len() == articles.deref().borrow().len() {
                "all"
            } else {
                "selected"
            };

            match download_bytes_as_file(&bytes, &format!("BibliZap-{suffix}-{timestamp}.ris")) {
                Ok(_) => (),
                Err(error) => {
                    gloo_console::log!(format!("{error}"));
                }
            }
        })
    };

    let on_bibtex_download_click = {
        let articles = articles.clone();
        let selected_articles = selected_articles.clone();
        Callback::from(move |_: MouseEvent| {
            let articles_to_download = get_articles_to_download(&articles, &selected_articles);
            let bytes = to_bibtex(&articles_to_download).unwrap();
            let timestamp = chrono::Local::now().to_rfc3339();
            let suffix = if articles_to_download.len() == articles.deref().borrow().len() {
                "all"
            } else {
                "selected"
            };

            match download_bytes_as_file(&bytes, &format!("BibliZap-{suffix}-{timestamp}.bib")) {
                Ok(_) => (),
                Err(error) => {
                    gloo_console::log!(format!("{error}"));
                }
            }
        })
    };
    html! {
        <div class="download-buttons">
            <DownloadButton onclick={on_excel_download_click} label="Excel"/>
            <DownloadButton onclick={on_ris_download_click} label="RIS"/>
            <DownloadButton onclick={on_bibtex_download_click} label="BibTeX"/>
        </div>
    }
}

/// Properties for the DownloadButton component.
#[derive(Clone, PartialEq, Properties)]
pub struct ButtonProps {
    pub label: String,
    pub onclick: Callback<MouseEvent>,
}

/// Component for the download button.
#[function_component]
pub fn DownloadButton(props: &ButtonProps) -> Html {
    html! {
        <div>
            <button class="btn btn-outline-secondary btn-lg mb-10" onclick={props.onclick.clone()}><i class="bi bi-download me-2"></i>{&props.label}</button>
        </div>
    }
}
