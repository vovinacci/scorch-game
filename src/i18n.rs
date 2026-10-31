//! Translations. Every piece of text on screen comes from a Fluent file in
//! `assets/i18n/` (compiled into the game), looked up by key with `t!`:
//!
//!     t!("menu-new-game")
//!     t!("income-note-bounty", n = player.wins, bounty = 50)
//!
//! Fluent handles plurals per language (Ukrainian has three forms). A key
//! missing in Ukrainian falls back to English; tests check that both files
//! have the same keys and that every key used in the code exists.

use std::cell::Cell;

use fluent_bundle::{FluentArgs, FluentBundle, FluentResource, FluentValue};
use unic_langid::LanguageIdentifier;

use crate::choice::choice;

choice!(Language { English, Ukrainian });

#[derive(Clone, Copy, PartialEq, Eq)]
enum Lang {
    En,
    Uk,
}

const EN: &str = include_str!("../assets/i18n/en-GB.ftl");
const UK: &str = include_str!("../assets/i18n/uk.ftl");

thread_local! {
    static CURRENT: Cell<Lang> = const { Cell::new(Lang::En) };
    static BUNDLE_EN: FluentBundle<FluentResource> = bundle("en-GB", EN);
    static BUNDLE_UK: FluentBundle<FluentResource> = bundle("uk", UK);
}

fn bundle(lang: &str, source: &str) -> FluentBundle<FluentResource> {
    let id: LanguageIdentifier = lang.parse().expect("valid language id");
    let mut bundle = FluentBundle::new(vec![id]);
    // No invisible Unicode isolation marks around arguments: the font has no
    // glyphs for them.
    bundle.set_use_isolating(false);
    let resource = FluentResource::try_new(source.to_string())
        .unwrap_or_else(|(_, errors)| panic!("bad {lang}.ftl: {errors:?}"));
    bundle
        .add_resource(resource)
        .unwrap_or_else(|errors| panic!("duplicate keys in {lang}.ftl: {errors:?}"));
    bundle
}

/// Switches the language of all text.
pub fn set_language(language: Language) {
    CURRENT.set(match language {
        Language::English => Lang::En,
        Language::Ukrainian => Lang::Uk,
    });
}

/// The operating system's language, if the game has it, else English. Used
/// once, on the first run; after that the player's choice is saved.
pub fn detect() -> Language {
    match sys_locale::get_locale() {
        Some(l) if l.to_lowercase().starts_with("uk") => Language::Ukrainian,
        _ => Language::English,
    }
}

/// Reads a saved language leniently: anything unknown (such as the old
/// "System") becomes `None`, so it's detected again instead of breaking the
/// whole profile.
pub fn lenient<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Language>, D::Error> {
    use serde::Deserialize;
    Ok(
        match Option::<String>::deserialize(d).ok().flatten().as_deref() {
            Some("English") => Some(Language::English),
            Some("Ukrainian") => Some(Language::Ukrainian),
            _ => None,
        },
    )
}

/// A value passed into a translation: text or a number (numbers pick plurals).
pub enum Arg {
    Text(String),
    Number(f64),
}

impl From<&str> for Arg {
    fn from(v: &str) -> Self {
        Arg::Text(v.to_string())
    }
}

impl From<String> for Arg {
    fn from(v: String) -> Self {
        Arg::Text(v)
    }
}

impl From<&String> for Arg {
    fn from(v: &String) -> Self {
        Arg::Text(v.clone())
    }
}

macro_rules! number_arg {
    ($($t:ty),+) => {
        $(impl From<$t> for Arg {
            fn from(v: $t) -> Self {
                Arg::Number(v as f64)
            }
        })+
    };
}
number_arg!(i32, u32, u64, usize, f32, f64);

/// Looks up `key` in the current language, falling back to English, then to
/// the key itself.
pub fn tr(key: &str, args: &[(&str, Arg)]) -> String {
    let mut fluent_args = FluentArgs::new();
    for (name, value) in args {
        let value = match value {
            Arg::Text(s) => FluentValue::from(s.clone()),
            Arg::Number(n) => FluentValue::from(*n),
        };
        fluent_args.set(*name, value);
    }
    let args = (!args.is_empty()).then_some(&fluent_args);
    let format = |bundle: &FluentBundle<FluentResource>| {
        let pattern = bundle.get_message(key)?.value()?;
        let mut errors = Vec::new();
        Some(
            bundle
                .format_pattern(pattern, args, &mut errors)
                .into_owned(),
        )
    };
    let found = match CURRENT.get() {
        Lang::Uk => BUNDLE_UK.with(format),
        Lang::En => None,
    };
    found
        .or_else(|| BUNDLE_EN.with(format))
        .unwrap_or_else(|| key.to_string())
}

/// Translated text: `t!(key)` or `t!(key, name = value, ...)`.
macro_rules! t {
    ($key:expr) => {
        $crate::i18n::tr($key, &[])
    };
    ($key:expr, $($name:ident = $value:expr),+ $(,)?) => {
        $crate::i18n::tr($key, &[$((stringify!($name), $crate::i18n::Arg::from($value))),+])
    };
}

pub(crate) use t;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn keys(source: &str) -> BTreeSet<String> {
        source
            .lines()
            .filter(|l| l.chars().next().is_some_and(|c| c.is_ascii_alphabetic()))
            .filter_map(|l| l.split_once(" =").map(|(k, _)| k.trim().to_string()))
            .collect()
    }

    #[test]
    fn both_languages_have_the_same_keys() {
        let (en, uk) = (keys(EN), keys(UK));
        let only_en: Vec<_> = en.difference(&uk).collect();
        let only_uk: Vec<_> = uk.difference(&en).collect();
        assert!(only_en.is_empty(), "missing in uk.ftl: {only_en:?}");
        assert!(only_uk.is_empty(), "missing in en-GB.ftl: {only_uk:?}");
    }

    #[test]
    fn both_files_parse() {
        BUNDLE_EN.with(|_| ());
        BUNDLE_UK.with(|_| ());
    }

    #[test]
    fn every_key_used_in_code_exists() {
        let en = keys(EN);
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src");
        let mut missing = Vec::new();
        for entry in std::fs::read_dir(dir).unwrap() {
            let code = std::fs::read_to_string(entry.unwrap().path()).unwrap();
            // A key right after `t!(`, but not inside `format!(` or `print!(`.
            for (at, _) in code.match_indices("t!(\"") {
                if code[..at].ends_with(|c: char| c.is_alphanumeric() || c == '_') {
                    continue;
                }
                let key = code[at + 4..].split('"').next().unwrap();
                if !en.contains(key) {
                    missing.push(key.to_string());
                }
            }
        }
        assert!(missing.is_empty(), "keys not in en-GB.ftl: {missing:?}");
    }

    /// Keys built at run time (`t!(&format!(...))`), which the code scan above can't see.
    #[test]
    fn keys_built_at_run_time_exist() {
        use crate::choice::Choice;
        fn choices<T: Choice>() -> Vec<String> {
            T::ALL.iter().map(|v| v.key().to_string()).collect()
        }
        let mut needed: Vec<String> = Vec::new();
        needed.extend(choices::<Language>());
        needed.extend(choices::<crate::settings::HealthDisplay>());
        needed.extend(choices::<crate::settings::GameSpeed>());
        needed.extend(choices::<crate::settings::BotSpeed>());
        needed.extend(choices::<crate::setup::SuddenDeath>());
        for bot in ["alpha", "bravo", "charlie"] {
            needed.push(format!("bot-{bot}"));
        }
        for reason in ["revenge", "leader", "weakest", "nearest"] {
            needed.push(format!("reason-{reason}"));
        }
        for name in crate::settings::CONTROLS {
            needed.push(format!("controls-{name}-key"));
            needed.push(format!("controls-{name}"));
        }
        needed.extend(
            crate::economy::Income::default()
                .lines()
                .map(|(k, _)| k.to_string()),
        );
        needed.extend(
            crate::stats::Stats::default()
                .rows()
                .into_iter()
                .map(|(k, _)| k.to_string()),
        );
        needed.extend(
            crate::setup::Setup::default()
                .summary(Some((1, 1)))
                .into_iter()
                .chain(crate::setup::Setup::default().summary(None))
                .map(|(k, _)| k.to_string()),
        );
        let en = keys(EN);
        let missing: Vec<_> = needed.iter().filter(|k| !en.contains(*k)).collect();
        assert!(missing.is_empty(), "keys not in en-GB.ftl: {missing:?}");
    }

    #[test]
    fn ukrainian_plurals_have_three_forms() {
        CURRENT.set(Lang::Uk);
        let forms: Vec<_> = [1, 3, 5, 21]
            .into_iter()
            .map(|n| tr("unit-wins", &[("n", Arg::from(n))]))
            .collect();
        CURRENT.set(Lang::En);
        assert_eq!(
            forms,
            ["1 перемога", "3 перемоги", "5 перемог", "21 перемога"] // ascii-ok: Ukrainian test data
        );
        assert_eq!(tr("unit-wins", &[("n", Arg::from(1))]), "1 win");
    }
}
