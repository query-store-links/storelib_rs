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
    /// When true, [`Self::dcat_trail`] emits the language as a BCP-47 tag
    /// (`<lang>-<market>`, e.g. `en-US`) instead of the bare ISO 639-1
    /// code (`en`).
    ///
    /// Microsoft returns *more* fields for the full-tag form — most
    /// notably `LocalizedProperties[].CMSVideos[]` (hero trailers,
    /// DASH/HLS URLs) is empty for `languages=en` but populated for
    /// `languages=en-US`. Default `false` for backwards-compatibility
    /// with [`Self::new`]; [`Self::production`] sets it to `true` since
    /// the richer response is what callers usually want.
    #[serde(default)]
    pub use_full_tag: bool,
}

impl Locale {
    /// Create a new `Locale` with [`Self::use_full_tag`] disabled. Use
    /// [`Self::with_full_tag`] to opt in to the BCP-47 form, or
    /// [`Self::production`] which enables it by default.
    pub fn new(market: Market, language: Lang, include_neutral: bool) -> Self {
        Locale {
            market,
            language,
            include_neutral,
            use_full_tag: false,
        }
    }

    /// Default production locale: `US / en`, neutral disabled, full-tag
    /// emission enabled (so requests carry `languages=en-US` and pick up
    /// CMS video metadata).
    pub fn production() -> Self {
        Locale {
            market: Market::Us,
            language: Lang::En,
            include_neutral: false,
            use_full_tag: true,
        }
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

    /// Toggle [`Self::use_full_tag`] fluently:
    ///
    /// ```
    /// use storelib_rs::{Lang, Locale, Market};
    /// let l = Locale::new(Market::Us, Lang::En, false).with_full_tag(true);
    /// assert!(l.dcat_trail().contains("languages=en-US"));
    /// ```
    #[must_use]
    pub fn with_full_tag(mut self, enabled: bool) -> Self {
        self.use_full_tag = enabled;
        self
    }

    /// Render the language component used in [`Self::dcat_trail`] —
    /// either `<lang>-<market>` (when [`Self::use_full_tag`] is true) or
    /// just `<lang>`.
    fn language_token(&self) -> String {
        if self.use_full_tag {
            format!("{}-{}", self.language.as_str(), self.market.as_str())
        } else {
            self.language.as_str().to_owned()
        }
    }

    /// Builds the trailing query-string fragment appended to DCat URLs.
    ///
    /// Examples:
    ///   `market=US&languages=en&catalogsource=apps`
    ///   `market=US&languages=en-US&catalogsource=apps` (full-tag enabled)
    ///   `market=DE&languages=de,en&catalogsource=apps` (with `include_neutral`)
    ///   `market=DE&languages=de-DE,en&catalogsource=apps` (both)
    pub fn dcat_trail(&self) -> String {
        let market = self.market.as_str();
        let lang = self.language_token();
        if self.include_neutral && self.language.as_str() != "en" {
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
        // production() enables full-tag so the DCat response includes
        // CMSVideos and other region-specific fields.
        let trail = Locale::production().dcat_trail();
        assert_eq!(trail, "market=US&languages=en-US&catalogsource=apps");
    }

    #[test]
    fn new_keeps_short_tag_for_backwards_compat() {
        // Locale::new() defaults use_full_tag=false so existing callers
        // keep their previous URL shape.
        let l = Locale::new(Market::Us, Lang::En, false);
        assert_eq!(l.dcat_trail(), "market=US&languages=en&catalogsource=apps");
        assert!(!l.use_full_tag);
    }

    #[test]
    fn with_full_tag_builder_switches_format() {
        let l = Locale::new(Market::Us, Lang::En, false).with_full_tag(true);
        assert_eq!(
            l.dcat_trail(),
            "market=US&languages=en-US&catalogsource=apps"
        );
        let l = l.with_full_tag(false);
        assert_eq!(l.dcat_trail(), "market=US&languages=en&catalogsource=apps");
    }

    #[test]
    fn full_tag_with_neutral_fallback() {
        // de-DE,en — the neutral fallback is always the bare `en`, never `en-US`.
        let l = Locale::new(Market::De, Lang::De, true).with_full_tag(true);
        assert_eq!(
            l.dcat_trail(),
            "market=DE&languages=de-DE,en&catalogsource=apps",
        );
    }

    #[test]
    fn full_tag_neutral_skipped_when_lang_is_en() {
        // en + full_tag → languages=en-US (no `,en` suffix, even though
        // include_neutral is true, because the primary lang is already en).
        let l = Locale::new(Market::Gb, Lang::En, true).with_full_tag(true);
        assert_eq!(
            l.dcat_trail(),
            "market=GB&languages=en-GB&catalogsource=apps"
        );
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
