use serde::{Deserialize, Serialize};

/// Top-level response from a DisplayCatalog autosuggest search.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all(serialize = "camelCase", deserialize = "PascalCase"))]
pub struct DCatSearch {
    pub results: Option<Vec<SearchResult>>,
    pub total_result_count: Option<i64>,
}

/// One result group returned by a DCat search.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all(serialize = "camelCase", deserialize = "PascalCase"))]
pub struct SearchResult {
    pub product_family_name: Option<String>,
    pub products: Option<Vec<crate::models::catalog::Product>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dcat_search_round_trip() {
        let json =
            r#"{"TotalResultCount":2,"Results":[{"ProductFamilyName":"Apps","Products":[]}]}"#;
        let s: DCatSearch = serde_json::from_str(json).unwrap();
        assert_eq!(s.total_result_count, Some(2));
        let results = s.results.as_deref().unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].product_family_name.as_deref(), Some("Apps"));

        let out = serde_json::to_string(&s).unwrap();
        assert!(out.contains("\"totalResultCount\":2"), "got: {out}");
        assert!(out.contains("\"results\":"), "got: {out}");
        assert!(out.contains("\"productFamilyName\":\"Apps\""), "got: {out}");
        assert!(!out.contains("\"TotalResultCount\""), "got: {out}");
        assert!(!out.contains("\"ProductFamilyName\""), "got: {out}");
    }

    #[test]
    fn search_result_empty_serializes_camel_case() {
        let r = SearchResult::default();
        let out = serde_json::to_string(&r).unwrap();
        // Both fields are Option<...>; default is None, which serde emits as null.
        assert!(out.contains("\"productFamilyName\":null"), "got: {out}");
        assert!(out.contains("\"products\":null"), "got: {out}");
    }
}
