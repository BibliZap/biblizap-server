use serde::de::{self, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Map;

use super::lensid::LensId;

/// Represents an article as returned by the Lens.org API.
///
/// This struct mirrors the structure of the article objects in the Lens.org API response.
#[derive(Serialize, Deserialize, Debug)]
pub struct Article {
    /// The Lens.org specific ID for the article.
    pub lens_id: LensId,
    /// The title of the article.
    pub title: Option<String>,
    /// The abstract or summary of the article.
    #[serde(rename = "abstract")]
    pub summary: Option<String>,
    /// The number of scholarly citations this article has received.
    pub scholarly_citations_count: Option<i32>,

    /// External identifiers for the article (e.g., DOI, PMID).
    /// When deserializing from Lens.org API, uses custom visitor to parse array format.
    #[serde(deserialize_with = "deserialize_external_ids_option")]
    pub external_ids: Option<ExternalIds>,
    /// The list of authors.
    pub authors: Option<Vec<Author>>,
    /// Information about the source (e.g., journal, conference).
    pub source: Option<Source>,
    /// The year of publication.
    pub year_published: Option<i32>,
}

/// Article metadata without the LensId.
///
/// This contains all the article fields except the lens_id itself.
/// Used in combination with ArticleWithData for completion results.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ArticleData {
    /// The title of the article.
    pub title: Option<String>,
    /// The abstract or summary of the article.
    #[serde(rename = "abstract")]
    pub summary: Option<String>,
    /// The number of scholarly citations this article has received.
    pub scholarly_citations_count: Option<i32>,

    /// External identifiers for the article (e.g., DOI, PMID).
    /// When deserializing from Lens.org API, uses custom visitor to parse array format.
    #[serde(deserialize_with = "deserialize_external_ids_option")]
    pub external_ids: Option<ExternalIds>,
    /// The list of authors.
    pub authors: Option<Vec<Author>>,
    /// Information about the source (e.g., journal, conference).
    pub source: Option<Source>,
    /// The year of publication.
    pub year_published: Option<i32>,
}

/// Article data combined with its LensId.
///
/// This is the primary structure returned from article completion operations.
/// The LensId is separate from the article data to make the key explicit.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ArticleWithData {
    /// The Lens.org specific ID for the article.
    pub lens_id: LensId,
    /// The article metadata.
    pub article_data: ArticleData,
}

/// Represents external identifiers for an article.
///
/// When stored in cache, this is serialized as a normal struct.
/// When read from Lens.org API, it's parsed from an array format via custom deserializer.
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct ExternalIds {
    /// List of PubMed IDs (PMID).
    pub pmid: Vec<String>,
    /// List of DOIs (Digital Object Identifier).
    pub doi: Vec<String>,
}

/// Represents an author in the Lens.org API response.
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Author {
    /// The first name of the author.
    pub first_name: Option<String>,
    /// The initials of the author.
    pub initials: Option<String>,
    /// The last name of the author.
    pub last_name: Option<String>,
}

/// Represents the source (e.g., journal) in the Lens.org API response.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Source {
    /// The publisher of the source.
    pub publisher: Option<String>,
    /// The title of the source (e.g., journal title).
    pub title: Option<String>,
    /// The type of source (e.g., "journal").
    #[serde(rename = "type")]
    pub kind: Option<String>,
}

/// Custom deserialization function for Option<ExternalIds> that handles both formats.
///
/// - API format: `[{"type": "doi", "value": "..."}]` (uses Visitor)
/// - Cache format: `{"pmid": [...], "doi": [...]}` (normal deserialization)
pub fn deserialize_external_ids_option<'de, D>(
    deserializer: D,
) -> Result<Option<ExternalIds>, D::Error>
where
    D: Deserializer<'de>,
{
    struct OptionalExternalIdsVisitor;

    impl<'de> Visitor<'de> for OptionalExternalIdsVisitor {
        type Value = Option<ExternalIds>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(
                formatter,
                "null, an object, or an array of external ID objects"
            )
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: Deserializer<'de>,
        {
            deserializer.deserialize_any(ExternalIdsFlexibleVisitor)
        }
    }

    deserializer.deserialize_option(OptionalExternalIdsVisitor)
}

/// A visitor that can handle both API array format and cache object format.
struct ExternalIdsFlexibleVisitor;

impl<'de> Visitor<'de> for ExternalIdsFlexibleVisitor {
    type Value = Option<ExternalIds>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            formatter,
            "an array of external ID objects or a struct with external ID fields"
        )
    }

    // Handle API format: array of {"type": "...", "value": "..."}
    fn visit_seq<V>(self, mut seq: V) -> Result<Self::Value, V::Error>
    where
        V: SeqAccess<'de>,
    {
        let mut result = ExternalIds::default();
        while let Some(value) = seq.next_element()? {
            let map: Map<String, serde_json::Value> = value;
            let value_type = map
                .get("type")
                .ok_or_else(|| de::Error::missing_field("type"))?
                .as_str()
                .ok_or_else(|| de::Error::custom("failed to get type string"))?;

            let value_str = map
                .get("value")
                .ok_or_else(|| de::Error::missing_field("value"))?
                .as_str()
                .ok_or_else(|| de::Error::custom("failed to get value string"))?
                .to_owned();

            match value_type {
                "pmid" => result.pmid.push(value_str),
                "doi" => result.doi.push(value_str),
                _ => {} // Ignore unknown types
            }
        }
        Ok(Some(result))
    }

    // Handle cache format: normal struct {"pmid": [...], "doi": [...], ...}
    fn visit_map<M>(self, map: M) -> Result<Self::Value, M::Error>
    where
        M: de::MapAccess<'de>,
    {
        let external_ids = ExternalIds::deserialize(de::value::MapAccessDeserializer::new(map))?;
        Ok(Some(external_ids))
    }
}

impl Article {
    /// Gets the full name of the first author, if available.
    ///
    /// Returns `None` if there are no authors or the first author's name is incomplete.
    pub fn first_author_name(&self) -> Option<String> {
        let authors = self.authors.clone()?;
        let author = authors.first()?;

        Some(format!(
            "{} {}",
            author.first_name.clone().unwrap_or_default(),
            author.last_name.clone().unwrap_or_default()
        ))
    }

    /// Gets the first PMID (PubMed ID) from the external identifiers, if available.
    ///
    /// Note: This currently incorrectly returns the first DOI. It should return the first PMID.
    pub fn pmid(&self) -> Option<String> {
        let external_ids = self.external_ids.clone()?;
        // TODO: This should return the first PMID, not DOI
        let id = external_ids.doi.first()?.to_owned();
        Some(id)
    }

    /// Gets the first DOI (Digital Object Identifier) from the external identifiers, if available.
    pub fn doi(&self) -> Option<String> {
        let external_ids = self.external_ids.clone()?;
        let id = external_ids.doi.first()?.to_owned();
        Some(id)
    }

    /// Gets the title of the source (e.g., journal title), if available.
    pub fn journal(&self) -> Option<String> {
        let source = self.source.clone()?;

        source.title
    }
}
