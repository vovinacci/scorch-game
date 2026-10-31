//! Settings with a fixed list of values, shown on screen as "< value >" selectors.

/// A setting with a fixed list of values.
pub trait Choice: Copy + PartialEq + 'static {
    const ALL: &'static [Self];
    /// Translation key, `Type-Variant`.
    fn key(self) -> &'static str;
    /// Name on screen, in the current language.
    fn text(self) -> String {
        crate::i18n::tr(self.key(), &[])
    }
}

/// Declares an enum that implements `Choice`: `choice!(Walls { Open, Solid });`
/// Names on screen come from the translation files, keys `Walls-Open`, ...
macro_rules! choice {
    ($name:ident { $($variant:ident),+ $(,)? }) => {
        #[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
        pub enum $name {
            $($variant),+
        }

        impl $crate::choice::Choice for $name {
            const ALL: &'static [Self] = &[$(Self::$variant),+];
            fn key(self) -> &'static str {
                match self {
                    $(Self::$variant => concat!(stringify!($name), "-", stringify!($variant))),+
                }
            }
        }
    };
}

pub(crate) use choice;
