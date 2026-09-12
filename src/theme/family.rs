use std::{borrow::Cow, fmt};

use super::{SymbolFamily, ThemeVariant};

/// Stable application-defined identity for a theme family.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ThemeId(String);

impl ThemeId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ThemeId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// The visual representations of one theme across terminal classes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThemeFamily {
    pub id: ThemeId,
    pub metadata: ThemeMetadata,
    pub full_color: ThemeVariant,
    pub color: ThemeVariant,
    pub styled_monochrome: ThemeVariant,
    pub plain: ThemeVariant,
    pub symbols: SymbolFamily,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ThemeMetadata {
    pub name: Cow<'static, str>,
    pub description: Option<Cow<'static, str>>,
}

impl ThemeFamily {
    pub fn new(
        id: ThemeId,
        full_color: ThemeVariant,
        color: ThemeVariant,
        styled_monochrome: ThemeVariant,
        plain: ThemeVariant,
    ) -> Self {
        let name = Cow::Owned(id.as_str().to_owned());
        Self {
            id,
            metadata: ThemeMetadata {
                name,
                description: None,
            },
            full_color,
            color,
            styled_monochrome,
            plain,
            symbols: SymbolFamily::default(),
        }
    }
}
