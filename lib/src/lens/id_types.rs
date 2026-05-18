use crate::lens::error::LensError;

/// A helper struct to categorize raw string IDs into known types (PMID, Lens ID, DOI).
pub struct TypedIdList<'a> {
    /// List of potential PubMed IDs.
    pub pmid: Vec<&'a str>,
    /// List of potential Lens.org IDs.
    pub lens_id: Vec<&'a str>,
    /// List of potential DOIs.
    pub doi: Vec<&'a str>,
}

impl<'a> TypedIdList<'a> {
    /// Categorizes a list of raw string IDs into known types using regular expressions.
    ///
    /// # Arguments
    ///
    /// * `id_list`: An iterator over string slices representing potential IDs.
    ///
    /// # Returns
    ///
    /// A `TypedIdList` containing the categorized IDs.
    pub fn from_raw_id_list<I>(id_list: I) -> Result<Self, LensError>
    where
        I: IntoIterator<Item = &'a str> + Clone,
    {
        use regex::Regex;
        // Regex for matching PMIDs (digits only)
        let pmid_regex = Regex::new("^[0-9]+$").expect("Failed to create PMID regex");
        // Regex for matching Lens IDs (format like XXX-XXX-...)
        let lens_id_regex =
            Regex::new("^...-...-...-...-...$").expect("Failed to create Lens ID regex");
        // Regex for matching DOIs (starts with 10.)
        let doi_regex = Regex::new("^10\\.").expect("Failed to create DOI regex");

        let pmid = id_list
            .clone()
            .into_iter()
            .filter(|n| pmid_regex.is_match(n))
            .collect::<Vec<_>>();

        let lens_id = id_list
            .clone()
            .into_iter()
            .filter(|n| lens_id_regex.is_match(n))
            .collect::<Vec<_>>();

        let doi = id_list
            .into_iter()
            .filter(|n| doi_regex.is_match(n))
            .collect::<Vec<&str>>();

        if pmid.is_empty() && lens_id.is_empty() && doi.is_empty() {
            return Err(LensError::NoValidIdsInInputList);
        }

        Ok(Self { pmid, lens_id, doi })
    }
}
