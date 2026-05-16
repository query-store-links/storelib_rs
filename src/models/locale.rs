//! Locale types backed by ISO 3166-1 alpha-2 and ISO 639-1.
//!
//! Re-run `tools/gen-locales.mjs` after updating the source data files in
//! `data/` to regenerate the `Market` and `Lang` enums.

mod iso_codes {
    include!("iso_codes.rs");
}

pub use iso_codes::{Lang, LanguageTag, Market};

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

    /// Build a [`Locale`] from a Microsoft Store BCP-47 [`LanguageTag`].
    ///
    /// Errors when the tag has no resolvable region (e.g. `zh-Hant`, `en-053`)
    /// or its primary language subtag is not ISO 639-1 (e.g. `chr-Cher-US`).
    /// `include_neutral` is passed through unchanged.
    pub fn from_tag(tag: LanguageTag, include_neutral: bool) -> Result<Self, &'static str> {
        let lang = tag
            .lang()
            .ok_or("language tag has no ISO 639-1 primary subtag")?;
        let market = tag
            .region()
            .ok_or("language tag has no ISO 3166-1 region subtag")?;
        Ok(Locale::new(market, lang, include_neutral))
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
    fn locale_from_tag() {
        let l = Locale::from_tag(LanguageTag::EnUs, false).unwrap();
        assert_eq!(l.market, Market::Us);
        assert_eq!(l.language, Lang::En);

        let l = Locale::from_tag(LanguageTag::ZhHantTw, true).unwrap();
        assert_eq!(l.market, Market::Tw);
        assert_eq!(l.language, Lang::Zh);
        assert!(l.include_neutral);

        // No region: zh-Hant alone
        assert!(Locale::from_tag(LanguageTag::ZhHant, false).is_err());
        // Non-ISO-639-1 primary lang: chr (Cherokee)
        assert!(Locale::from_tag(LanguageTag::ChrCherUs, false).is_err());
        // UN M.49 numeric region: en-053 (not in Market enum)
        assert!(Locale::from_tag(LanguageTag::En053, false).is_err());
    }

    #[test]
    fn locale_from_tag_dcat_trail_picks_lang_not_tag() {
        // Locale takes the 2-letter language; the script/variant subtags are
        // dropped. zh-Hant-TW collapses to (TW, zh).
        let l = Locale::from_tag(LanguageTag::ZhHantTw, false).unwrap();
        assert_eq!(l.dcat_trail(), "market=TW&languages=zh&catalogsource=apps",);
    }

    #[test]
    fn locale_from_tag_three_segment_with_script_and_region() {
        // az-Latn-AZ: az + Latn (script) + AZ (region)
        let l = Locale::from_tag(LanguageTag::AzLatnAz, false).unwrap();
        assert_eq!(l.market, Market::Az);
        assert_eq!(l.language, Lang::Az);

        // sr-Cyrl-RS
        let l = Locale::from_tag(LanguageTag::SrCyrlRs, true).unwrap();
        assert_eq!(l.market, Market::Rs);
        assert_eq!(l.language, Lang::Sr);
    }

    #[test]
    fn locale_from_tag_variant_subtag() {
        // ca-ES-valencia has a variant subtag after the region; both still
        // resolve.
        let l = Locale::from_tag(LanguageTag::CaEsValencia, false).unwrap();
        assert_eq!(l.market, Market::Es);
        assert_eq!(l.language, Lang::Ca);
    }

    #[test]
    fn language_tag_lang_projection() {
        // Plain language: en → en
        assert_eq!(LanguageTag::En.lang(), Some(Lang::En));
        // Language with region: en-US → en
        assert_eq!(LanguageTag::EnUs.lang(), Some(Lang::En));
        // Language with script: zh-Hant → zh
        assert_eq!(LanguageTag::ZhHant.lang(), Some(Lang::Zh));
        // Language with script + region: zh-Hant-TW → zh
        assert_eq!(LanguageTag::ZhHantTw.lang(), Some(Lang::Zh));
        // 3-letter primary: chr is not ISO 639-1
        assert_eq!(LanguageTag::ChrCher.lang(), None);
        assert_eq!(LanguageTag::FilPh.lang(), None);
        assert_eq!(LanguageTag::PrsAf.lang(), None);
    }

    #[test]
    fn language_tag_region_projection() {
        // No region subtag
        assert_eq!(LanguageTag::En.region(), None);
        assert_eq!(LanguageTag::ZhHant.region(), None);
        // Region present
        assert_eq!(LanguageTag::EnUs.region(), Some(Market::Us));
        assert_eq!(LanguageTag::ZhHantTw.region(), Some(Market::Tw));
        assert_eq!(LanguageTag::SrCyrlBa.region(), Some(Market::Ba));
        // UN M.49 numeric region — not in the Market enum
        assert_eq!(LanguageTag::En053.region(), None);
        assert_eq!(LanguageTag::Es419.region(), None);
    }

    #[test]
    fn locale_dcat_trail_non_en_with_neutral() {
        // Non-English language with neutral fallback enabled appends `,en`.
        let l = Locale::new(Market::Jp, Lang::Ja, true);
        assert_eq!(
            l.dcat_trail(),
            "market=JP&languages=ja,en&catalogsource=apps",
        );
    }

    #[test]
    fn locale_dcat_trail_non_en_without_neutral() {
        let l = Locale::new(Market::Jp, Lang::Ja, false);
        assert_eq!(l.dcat_trail(), "market=JP&languages=ja&catalogsource=apps",);
    }

    #[test]
    fn language_tag_roundtrip() {
        assert_eq!(LanguageTag::EnUs.as_str(), "en-US");
        assert_eq!(LanguageTag::ZhHant.as_str(), "zh-Hant");
        assert_eq!(LanguageTag::SrCyrlRs.as_str(), "sr-Cyrl-RS");
        assert_eq!(LanguageTag::CaEsValencia.as_str(), "ca-ES-valencia");
        assert_eq!(LanguageTag::from_str("en-us").unwrap(), LanguageTag::EnUs);
        assert_eq!(
            LanguageTag::from_str("ZH-HANT").unwrap(),
            LanguageTag::ZhHant,
        );
        assert!(LanguageTag::from_str("xx-YY").is_err());
    }

    #[test]
    fn english_names() {
        assert_eq!(Market::Us.english_name(), "United States");
        assert_eq!(Lang::En.english_name(), "English");
        assert_eq!(LanguageTag::EnUs.english_name(), "English");
        assert_eq!(LanguageTag::ZhHant.english_name(), "Chinese (Traditional)",);
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
