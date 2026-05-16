//! Locale types backed by ISO 3166-1 alpha-2 and ISO 639-1.
//!
//! Re-run `tools/gen-locales.mjs` after updating the source data files in
//! `data/` to regenerate the `Market` and `Lang` enums.

mod iso_codes {
    include!("iso_codes.rs");
}

pub use iso_codes::{Lang, Market};

/// Combined locale used when forming DisplayCatalog request URLs.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Locale {
    pub market: Market,
    pub language: Lang,
    pub include_neutral: bool,
}

impl Locale {
    /// Create a new `Locale`.
    pub fn new(market: Market, language: Lang, include_neutral: bool) -> Self {
        Locale {
            market,
            language,
            include_neutral,
        }
    }

    /// Default production locale: `US / en`, neutral disabled (`en` is already
    /// the neutral fallback).
    pub fn production() -> Self {
        Locale::new(Market::Us, Lang::En, false)
    }

    /// Builds the trailing query-string fragment appended to DCat URLs.
    ///
    /// Examples:
    ///   `market=US&languages=en&catalogsource=apps`
    ///   `market=DE&languages=de,en&catalogsource=apps` (with `include_neutral`)
    pub fn dcat_trail(&self) -> String {
        let market = self.market.as_str();
        let lang = self.language.as_str();
        if self.include_neutral && lang != "en" {
            format!("market={market}&languages={lang},en&catalogsource=apps")
        } else {
            format!("market={market}&languages={lang}&catalogsource=apps")
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn production_locale_trail() {
        let trail = Locale::production().dcat_trail();
        assert_eq!(trail, "market=US&languages=en&catalogsource=apps");
    }

    #[test]
    fn locale_with_neutral_fallback() {
        let locale = Locale::new(Market::De, Lang::De, true);
        assert_eq!(
            locale.dcat_trail(),
            "market=DE&languages=de,en&catalogsource=apps",
        );
    }

    #[test]
    fn locale_neutral_skipped_when_lang_is_en() {
        let locale = Locale::new(Market::Us, Lang::En, true);
        assert_eq!(
            locale.dcat_trail(),
            "market=US&languages=en&catalogsource=apps",
        );
    }

    #[test]
    fn market_roundtrip() {
        assert_eq!(Market::Us.as_str(), "US");
        assert_eq!(Market::Jp.as_str(), "JP");
        assert_eq!(Market::Zw.as_str(), "ZW");
        assert_eq!(Market::from_str("us").unwrap(), Market::Us);
        assert_eq!(Market::from_str("ZW").unwrap(), Market::Zw);
        assert!(Market::from_str("XX").is_err());
    }

    #[test]
    fn lang_roundtrip() {
        assert_eq!(Lang::En.as_str(), "en");
        assert_eq!(Lang::Zh.as_str(), "zh");
        assert_eq!(Lang::from_str("EN").unwrap(), Lang::En);
        assert!(Lang::from_str("xx").is_err());
    }

    #[test]
    fn english_names() {
        // Names come straight from the ISO source data — the parenthetical
        // suffix on some country names (e.g. "(the)") is part of the standard.
        assert_eq!(Market::Us.english_name(), "United States of America (the)");
        assert_eq!(Lang::En.english_name(), "English");
    }

    #[test]
    fn serde_uses_canonical_code() {
        assert_eq!(serde_json::to_string(&Market::Us).unwrap(), "\"US\"",);
        assert_eq!(
            serde_json::from_str::<Market>("\"GB\"").unwrap(),
            Market::Gb,
        );
        assert_eq!(serde_json::to_string(&Lang::En).unwrap(), "\"en\"",);
    }
}
