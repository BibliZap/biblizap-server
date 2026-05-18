use serde::de::{self, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use std::collections::HashMap;
use std::marker::PhantomData;

use crate::lens::error::LensError;

use super::article::{ExternalIds, deserialize_external_ids_option};
use super::lensid::LensId;

/// Maximum number of citations/references to deserialize per article.
///
/// This limit prevents memory issues with highly-cited papers (e.g., papers with 120k+ citations).
/// Only affects ~0.1% of papers with more than 300 citations/references.
///
/// Note: The Lens API returns citations/references sorted by lens_id (essentially random order),
/// so this gives a representative sample rather than systematic bias.
pub const MAX_RELATIONSHIPS_PER_ARTICLE: usize = 200;

/// Represents a list of references returned by the Lens.org API.
///
/// This struct is used for deserializing the `references` field, which is
/// expected to be a sequence of objects containing a `lens_id`.
///
/// **Limit**: Deserialization stops after `MAX_CITATIONS_PER_ARTICLE` items to prevent
/// memory issues with highly-cited papers.
#[derive(Debug, Default)]
pub struct References(pub Vec<LensId>);

impl<'de> Deserialize<'de> for References {
    /// Custom deserialization logic for the `References` struct.
    ///
    /// This is needed because the API returns a list of objects like
    /// `{"lens_id": "..."}` instead of just a list of Lens IDs.
    ///
    /// Stops deserializing after `MAX_CITATIONS_PER_ARTICLE` items.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let visitor = ReferencesVisitor(PhantomData);
        deserializer.deserialize_seq(visitor)
    }
}

/// A visitor for deserializing the `references` field from the Lens.org API.
///
/// This field is returned as a sequence of objects, each expected to contain a "lens_id" field.
///
/// **Limit**: Stops deserializing after `MAX_CITATIONS_PER_ARTICLE` items.
struct ReferencesVisitor(PhantomData<fn() -> References>);
impl<'de> Visitor<'de> for ReferencesVisitor {
    type Value = References;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a vec of objects with a 'lens_id' field")
    }

    /// Visits a sequence of reference objects and extracts the Lens IDs.
    ///
    /// Stops after `MAX_CITATIONS_PER_ARTICLE` items to prevent memory issues.
    fn visit_seq<V>(self, mut seq: V) -> Result<References, V::Error>
    where
        V: SeqAccess<'de>,
    {
        let mut out = References::default();
        let mut count = 0;

        while count < MAX_RELATIONSHIPS_PER_ARTICLE {
            match seq.next_element()? {
                Some(value) => {
                    let map: serde_json::Map<String, serde_json::Value> = value;
                    let lensid_value = map.get("lens_id");
                    if let Some(lensid_value) = lensid_value {
                        let lensid_str = lensid_value.as_str().ok_or_else(|| {
                            de::Error::custom("error converting lensid value to string")
                        })?;
                        let lensid = LensId::try_from(lensid_str)
                            .ok()
                            .ok_or_else(|| de::Error::custom("invalid lensid"))?;
                        out.0.push(lensid);
                        count += 1;
                    }
                }
                None => break,
            }
        }

        // Consume and discard remaining elements
        while seq.next_element::<serde_json::Value>()?.is_some() {}

        Ok(out)
    }
}

/// Represents a list of scholarly citations returned by the Lens.org API.
///
/// This struct is used for deserializing the `scholarly_citations` field, which is
/// expected to be a list of Lens IDs.
///
/// **Limit**: Deserialization stops after `MAX_CITATIONS_PER_ARTICLE` items to prevent
/// memory issues with highly-cited papers.
#[derive(Debug, Default)]
pub struct ScholarlyCitations(pub Vec<LensId>);

impl<'de> Deserialize<'de> for ScholarlyCitations {
    /// Custom deserialization logic that limits to `MAX_CITATIONS_PER_ARTICLE` items.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let visitor = ScholarlyCitationsVisitor(PhantomData);
        deserializer.deserialize_seq(visitor)
    }
}

/// A visitor for deserializing the `scholarly_citations` field with a limit.
struct ScholarlyCitationsVisitor(PhantomData<fn() -> ScholarlyCitations>);
impl<'de> Visitor<'de> for ScholarlyCitationsVisitor {
    type Value = ScholarlyCitations;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a vec of lens_id strings")
    }

    /// Visits a sequence of Lens IDs.
    ///
    /// Stops after `MAX_CITATIONS_PER_ARTICLE` items to prevent memory issues.
    fn visit_seq<V>(self, mut seq: V) -> Result<ScholarlyCitations, V::Error>
    where
        V: SeqAccess<'de>,
    {
        let mut out = ScholarlyCitations::default();
        let mut count = 0;

        while count < MAX_RELATIONSHIPS_PER_ARTICLE {
            match seq.next_element()? {
                Some(lens_id) => {
                    out.0.push(lens_id);
                    count += 1;
                }
                None => break,
            }
        }

        // Consume and discard remaining elements
        while seq.next_element::<LensId>()?.is_some() {}

        Ok(out)
    }
}

/// Combines references and scholarly citations lists from the Lens.org API response.
#[derive(Debug, Default, Deserialize)]
pub struct ReferencesAndCitations {
    /// The list of references cited by the article.
    #[serde(default)]
    pub references: References,
    /// The list of scholarly citations that cite the article.
    #[serde(default)]
    pub scholarly_citations: ScholarlyCitations,
}

impl ReferencesAndCitations {
    /// Gets a combined list of Lens IDs from both references and scholarly citations.
    pub fn get_both(&self) -> Vec<LensId> {
        let references = self.references.0.iter().map(|n| n.to_owned());
        let scholarly_citations = self.scholarly_citations.0.iter().map(|n| n.to_owned());

        references.chain(scholarly_citations).collect()
    }
}

#[derive(Debug, Deserialize)]
pub struct ArticleWithReferencesAndCitations {
    pub lens_id: LensId,
    #[serde(flatten)]
    pub refs_and_cites: ReferencesAndCitations,
    /// External IDs (PMID, DOI, etc.) - needed for ID mapping when querying by non-LensId
    #[serde(default, deserialize_with = "deserialize_external_ids_option")]
    pub external_ids: Option<ExternalIds>,
}

impl ArticleWithReferencesAndCitations {
    pub fn id_mappings_single_article(&self) -> Option<HashMap<String, LensId>> {
        let external_ids = self.external_ids.clone()?;

        let pmid = external_ids
            .pmid
            .clone()
            .into_iter()
            .map(|x| (x, self.lens_id.clone()));

        let doi = external_ids
            .doi
            .clone()
            .into_iter()
            .map(|x| (x, self.lens_id.clone()));

        Some(pmid.chain(doi).collect())
    }

    pub fn id_mappings<'a, I>(articles: I) -> HashMap<String, LensId>
    where
        I: IntoIterator<Item = &'a Self>,
    {
        articles
            .into_iter()
            .filter_map(|article| article.id_mappings_single_article())
            .fold(HashMap::new(), |mut acc, x| {
                acc.extend(x);
                acc
            })
    }

    pub async fn store_any_mappings<'a, I>(
        articles: I,
        cache: Option<&dyn super::cache::CacheBackend>,
    ) -> Result<(), LensError>
    where
        I: IntoIterator<Item = &'a Self>,
    {
        if let Some(cache_backend) = cache {
            let mappings: Vec<(String, LensId)> =
                ArticleWithReferencesAndCitations::id_mappings(articles)
                    .into_iter()
                    .collect();
            cache_backend.store_id_mapping(&mappings).await?;
        }
        Ok(())
    }
}

/// Helper struct to preserve parent-child relationship when querying citations.
#[derive(Debug)]
pub struct ArticleWithReferencesAndCitationsMerged {
    pub parent_id: LensId,
    pub children: Vec<LensId>,
}

impl From<ArticleWithReferencesAndCitations> for ArticleWithReferencesAndCitationsMerged {
    fn from(article: ArticleWithReferencesAndCitations) -> Self {
        Self {
            parent_id: article.lens_id,
            children: article.refs_and_cites.get_both(),
        }
    }
}

impl From<(LensId, Vec<LensId>)> for ArticleWithReferencesAndCitationsMerged {
    fn from((parent_id, children): (LensId, Vec<LensId>)) -> Self {
        Self {
            parent_id,
            children,
        }
    }
}
