#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Language {
    #[default]
    Zh,
    En,
}

impl Language {
    pub fn key(self) -> &'static str {
        match self {
            Self::Zh => "zh",
            Self::En => "en",
        }
    }

    pub fn parse(value: &str) -> Self {
        if value.eq_ignore_ascii_case("en") {
            Self::En
        } else {
            Self::Zh
        }
    }

    pub fn text<'a>(self, zh: &'a str, en: &'a str) -> &'a str {
        match self {
            Self::Zh => zh,
            Self::En => en,
        }
    }

    pub fn is_en(self) -> bool {
        self == Self::En
    }
}

#[cfg(test)]
mod tests {
    use super::Language;

    #[test]
    fn persisted_language_keys_round_trip() {
        for language in [Language::Zh, Language::En] {
            assert_eq!(Language::parse(language.key()), language);
        }
    }
}
