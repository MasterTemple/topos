//! Strong's numbers.

/// Strong's language
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum StrongsLang {
    Hebrew,
    Greek,
}

/// A Strong's number such as `H7225` or `G25`, with an optional suffix (`H0430a` → `a`)
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Strongs {
    pub lang: StrongsLang,
    pub number: u32,
    pub suffix: Option<String>,
}

/// The prefixes a token may have before its language letter; at most one is removed
const PREFIXES: &[&str] = &["strong:", "x-Strongs:", "Strongs:"];

impl Strongs {
    /// Parses one token: `strong:H07225`, `x-Strongs:G25`, `G25`, `H0430a`
    pub fn parse(token: &str) -> Option<Strongs> {
        let rest = PREFIXES
            .iter()
            .find_map(|p| token.strip_prefix(p))
            .unwrap_or(token);
        let mut chars = rest.chars();
        let lang = match chars.next()? {
            'G' | 'g' => StrongsLang::Greek,
            'H' | 'h' => StrongsLang::Hebrew,
            _ => return None,
        };
        let rest = chars.as_str();
        let digits = rest
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(rest.len());
        let (number, suffix) = rest.split_at(digits);
        if number.is_empty() {
            return None;
        }
        let number = number.parse::<u32>().ok()?;
        if !suffix
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '!')
        {
            return None;
        }
        Some(Strongs {
            lang,
            number,
            suffix: (!suffix.is_empty()).then(|| suffix.to_string()),
        })
    }

    /// Parses a whitespace-separated `lemma` attribute, keeping only Strong's tokens
    pub fn parse_list(lemma: &str) -> Vec<Strongs> {
        lemma
            .split_whitespace()
            .filter_map(Strongs::parse)
            .collect()
    }

    /// Key for looking the number up in SWORD's StrongsGreek/StrongsHebrew modules
    pub fn dictionary_key(&self) -> String {
        format!("{:05}", self.number)
    }

    /// Conventional dictionary module name: `StrongsGreek` / `StrongsHebrew`
    pub fn dictionary_module(&self) -> &'static str {
        match self.lang {
            StrongsLang::Greek => "StrongsGreek",
            StrongsLang::Hebrew => "StrongsHebrew",
        }
    }
}

impl std::fmt::Display for Strongs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let letter = match self.lang {
            StrongsLang::Greek => 'G',
            StrongsLang::Hebrew => 'H',
        };
        write!(f, "{letter}{}", self.number)?;
        if let Some(suffix) = &self.suffix {
            f.write_str(suffix)?;
        }
        Ok(())
    }
}
