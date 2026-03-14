/// All supported Store markets (two-letter country codes).
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Market {
    Af,
    Al,
    Dz,
    Ao,
    Ar,
    Am,
    Au,
    At,
    Az,
    Bs,
    Bh,
    Bd,
    Be,
    Bz,
    Bo,
    Ba,
    Bw,
    Br,
    Bn,
    Bg,
    Cm,
    Ca,
    Cv,
    Cl,
    Co,
    Cr,
    Hr,
    Cy,
    Cz,
    Dk,
    Do,
    Ec,
    Eg,
    Sv,
    Et,
    Ee,
    Fj,
    Fi,
    Fr,
    Ge,
    De,
    Gh,
    Gr,
    Gt,
    Hk,
    Hn,
    Hu,
    Is,
    In,
    Id,
    Iq,
    Ie,
    Il,
    It,
    Jm,
    Jp,
    Jo,
    Kz,
    Ke,
    Kw,
    Kg,
    Lv,
    Lb,
    Li,
    Lt,
    Lu,
    My,
    Mv,
    Mt,
    Mx,
    Mn,
    Ma,
    Mz,
    Ng,
    Ni,
    Np,
    Nl,
    Nz,
    /// Second NI entry (alias)
    Ni2,
    No,
    Om,
    Pk,
    Pa,
    Py,
    Pe,
    Ph,
    Pl,
    Pt,
    Qa,
    Ro,
    Ru,
    Sa,
    Sn,
    Sg,
    Sk,
    Si,
    Za,
    Kr,
    Es,
    Lk,
    Se,
    Ch,
    Tw,
    Tj,
    Tz,
    Th,
    Tt,
    Tn,
    Tr,
    Tm,
    Ug,
    Ua,
    Ae,
    Gb,
    Us,
    Uy,
    Uz,
    Ve,
    Vn,
    Ye,
    Zm,
    Zw,
}

impl Market {
    /// Returns the uppercase two-letter market string (e.g. `"US"`).
    pub fn as_str(&self) -> &'static str {
        match self {
            Market::Af => "AF",
            Market::Al => "AL",
            Market::Dz => "DZ",
            Market::Ao => "AO",
            Market::Ar => "AR",
            Market::Am => "AM",
            Market::Au => "AU",
            Market::At => "AT",
            Market::Az => "AZ",
            Market::Bs => "BS",
            Market::Bh => "BH",
            Market::Bd => "BD",
            Market::Be => "BE",
            Market::Bz => "BZ",
            Market::Bo => "BO",
            Market::Ba => "BA",
            Market::Bw => "BW",
            Market::Br => "BR",
            Market::Bn => "BN",
            Market::Bg => "BG",
            Market::Cm => "CM",
            Market::Ca => "CA",
            Market::Cv => "CV",
            Market::Cl => "CL",
            Market::Co => "CO",
            Market::Cr => "CR",
            Market::Hr => "HR",
            Market::Cy => "CY",
            Market::Cz => "CZ",
            Market::Dk => "DK",
            Market::Do => "DO",
            Market::Ec => "EC",
            Market::Eg => "EG",
            Market::Sv => "SV",
            Market::Et => "ET",
            Market::Ee => "EE",
            Market::Fj => "FJ",
            Market::Fi => "FI",
            Market::Fr => "FR",
            Market::Ge => "GE",
            Market::De => "DE",
            Market::Gh => "GH",
            Market::Gr => "GR",
            Market::Gt => "GT",
            Market::Hk => "HK",
            Market::Hn => "HN",
            Market::Hu => "HU",
            Market::Is => "IS",
            Market::In => "IN",
            Market::Id => "ID",
            Market::Iq => "IQ",
            Market::Ie => "IE",
            Market::Il => "IL",
            Market::It => "IT",
            Market::Jm => "JM",
            Market::Jp => "JP",
            Market::Jo => "JO",
            Market::Kz => "KZ",
            Market::Ke => "KE",
            Market::Kw => "KW",
            Market::Kg => "KG",
            Market::Lv => "LV",
            Market::Lb => "LB",
            Market::Li => "LI",
            Market::Lt => "LT",
            Market::Lu => "LU",
            Market::My => "MY",
            Market::Mv => "MV",
            Market::Mt => "MT",
            Market::Mx => "MX",
            Market::Mn => "MN",
            Market::Ma => "MA",
            Market::Mz => "MZ",
            Market::Ng => "NG",
            Market::Ni => "NI",
            Market::Np => "NP",
            Market::Nl => "NL",
            Market::Nz => "NZ",
            Market::Ni2 => "NI",
            Market::No => "NO",
            Market::Om => "OM",
            Market::Pk => "PK",
            Market::Pa => "PA",
            Market::Py => "PY",
            Market::Pe => "PE",
            Market::Ph => "PH",
            Market::Pl => "PL",
            Market::Pt => "PT",
            Market::Qa => "QA",
            Market::Ro => "RO",
            Market::Ru => "RU",
            Market::Sa => "SA",
            Market::Sn => "SN",
            Market::Sg => "SG",
            Market::Sk => "SK",
            Market::Si => "SI",
            Market::Za => "ZA",
            Market::Kr => "KR",
            Market::Es => "ES",
            Market::Lk => "LK",
            Market::Se => "SE",
            Market::Ch => "CH",
            Market::Tw => "TW",
            Market::Tj => "TJ",
            Market::Tz => "TZ",
            Market::Th => "TH",
            Market::Tt => "TT",
            Market::Tn => "TN",
            Market::Tr => "TR",
            Market::Tm => "TM",
            Market::Ug => "UG",
            Market::Ua => "UA",
            Market::Ae => "AE",
            Market::Gb => "GB",
            Market::Us => "US",
            Market::Uy => "UY",
            Market::Uz => "UZ",
            Market::Ve => "VE",
            Market::Vn => "VN",
            Market::Ye => "YE",
            Market::Zm => "ZM",
            Market::Zw => "ZW",
        }
    }
}

/// Supported UI languages.
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Lang {
    En,
    Ar,
    Az,
    Be,
    Bg,
    Bn,
    Bs,
    Ca,
    Cs,
    Da,
    De,
    El,
    EnGb,
    Es,
    EsMx,
    Et,
    Eu,
    Fa,
    Fi,
    Fr,
    FrCa,
    Gl,
    Gu,
    He,
    Hi,
    Hr,
    Hu,
    Hy,
    Id,
    Is,
    It,
    Ja,
    Ka,
    Kk,
    Km,
    Kn,
    Ko,
    Lt,
    Lv,
    Mk,
    Ml,
    Mr,
    Ms,
    Nb,
    Nl,
    Or,
    Pa,
    Pl,
    Pt,
    PtBr,
    Ro,
    Ru,
    Sk,
    Sl,
    Sr,
    SrLatn,
    Sv,
    Te,
    Tg,
    Th,
    Tr,
    Uk,
    Ur,
    Uz,
    Vi,
    ZhHant,
    ZhHans,
}

impl Lang {
    /// Returns the BCP-47 language tag (e.g. `"en-US"`, `"zh-Hant"`).
    pub fn as_str(&self) -> &'static str {
        match self {
            Lang::En => "en-US",
            Lang::Ar => "ar",
            Lang::Az => "az",
            Lang::Be => "be",
            Lang::Bg => "bg",
            Lang::Bn => "bn",
            Lang::Bs => "bs",
            Lang::Ca => "ca",
            Lang::Cs => "cs",
            Lang::Da => "da",
            Lang::De => "de",
            Lang::El => "el",
            Lang::EnGb => "en-GB",
            Lang::Es => "es",
            Lang::EsMx => "es-MX",
            Lang::Et => "et",
            Lang::Eu => "eu",
            Lang::Fa => "fa",
            Lang::Fi => "fi",
            Lang::Fr => "fr",
            Lang::FrCa => "fr-CA",
            Lang::Gl => "gl",
            Lang::Gu => "gu",
            Lang::He => "he",
            Lang::Hi => "hi",
            Lang::Hr => "hr",
            Lang::Hu => "hu",
            Lang::Hy => "hy",
            Lang::Id => "id",
            Lang::Is => "is",
            Lang::It => "it",
            Lang::Ja => "ja",
            Lang::Ka => "ka",
            Lang::Kk => "kk",
            Lang::Km => "km",
            Lang::Kn => "kn",
            Lang::Ko => "ko",
            Lang::Lt => "lt",
            Lang::Lv => "lv",
            Lang::Mk => "mk",
            Lang::Ml => "ml",
            Lang::Mr => "mr",
            Lang::Ms => "ms",
            Lang::Nb => "nb",
            Lang::Nl => "nl",
            Lang::Or => "or",
            Lang::Pa => "pa",
            Lang::Pl => "pl",
            Lang::Pt => "pt",
            Lang::PtBr => "pt-BR",
            Lang::Ro => "ro",
            Lang::Ru => "ru",
            Lang::Sk => "sk",
            Lang::Sl => "sl",
            Lang::Sr => "sr",
            Lang::SrLatn => "sr-Latn",
            Lang::Sv => "sv",
            Lang::Te => "te",
            Lang::Tg => "tg",
            Lang::Th => "th",
            Lang::Tr => "tr",
            Lang::Uk => "uk",
            Lang::Ur => "ur",
            Lang::Uz => "uz",
            Lang::Vi => "vi",
            Lang::ZhHant => "zh-Hant",
            Lang::ZhHans => "zh-Hans",
        }
    }
}

/// Combined locale used when forming DisplayCatalog request URLs.
#[derive(Debug, Clone)]
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

    /// Default production locale: `US / en-US`, neutral included.
    pub fn production() -> Self {
        Locale::new(Market::Us, Lang::En, true)
    }

    /// Returns the `market_str` component (uppercase country code).
    fn market_str(&self) -> &'static str {
        self.market.as_str()
    }

    /// Returns the BCP-47 language tag for the locale's language.
    fn lang_str(&self) -> &'static str {
        self.language.as_str()
    }

    /// Builds the trailing query-string fragment appended to DCat URLs.
    ///
    /// Example: `market=US&languages=en-US,en&catalogsource=apps`
    pub fn dcat_trail(&self) -> String {
        let market = self.market_str();
        let lang = self.lang_str();
        if self.include_neutral {
            format!("market={}&languages={},en&catalogsource=apps", market, lang)
        } else {
            format!("market={}&languages={}&catalogsource=apps", market, lang)
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn production_locale_trail() {
        let trail = Locale::production().dcat_trail();
        assert_eq!(trail, "market=US&languages=en-US,en&catalogsource=apps");
    }

    #[test]
    fn locale_without_neutral() {
        let locale = Locale::new(Market::Us, Lang::En, false);
        let trail = locale.dcat_trail();
        assert_eq!(trail, "market=US&languages=en-US&catalogsource=apps");
        assert!(!trail.contains(",en"));
    }

    #[test]
    fn locale_german_market() {
        let locale = Locale::new(Market::De, Lang::De, false);
        let trail = locale.dcat_trail();
        assert_eq!(trail, "market=DE&languages=de&catalogsource=apps");
    }

    #[test]
    fn locale_gb_english() {
        let locale = Locale::new(Market::Gb, Lang::EnGb, true);
        let trail = locale.dcat_trail();
        assert_eq!(trail, "market=GB&languages=en-GB,en&catalogsource=apps");
    }

    #[test]
    fn market_as_str_roundtrip() {
        assert_eq!(Market::Us.as_str(), "US");
        assert_eq!(Market::Jp.as_str(), "JP");
        assert_eq!(Market::Zw.as_str(), "ZW");
    }

    #[test]
    fn lang_as_str_roundtrip() {
        assert_eq!(Lang::En.as_str(), "en-US");
        assert_eq!(Lang::ZhHant.as_str(), "zh-Hant");
        assert_eq!(Lang::PtBr.as_str(), "pt-BR");
    }
}
