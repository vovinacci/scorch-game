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

    /// The `Random` value, which stands for "pick one each round" (wherever it is in the list).
    fn is_random(self) -> bool {
        self.key().ends_with("-Random")
    }

    /// A value other than `Random`, at random.
    fn any_real() -> Self {
        let real: Vec<Self> = Self::ALL
            .iter()
            .copied()
            .filter(|v| !v.is_random())
            .collect();
        real[crate::rng::gen_range(0, real.len())]
    }

    /// `self`, or for `Random` a real value picked at random.
    fn roll(self) -> Self {
        if self.is_random() {
            Self::any_real()
        } else {
            self
        }
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
