use serde::{Deserialize, Serialize};

/// Top-level response from a DisplayCatalog autosuggest search.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct DCatSearch {
    pub results: Option<Vec<SearchResult>>,
    pub total_result_count: Option<i64>,
}

/// One result group returned by a DCat search.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct SearchResult {
    pub product_family_name: Option<String>,
    pub products: Option<Vec<crate::models::catalog::Product>>,
}
