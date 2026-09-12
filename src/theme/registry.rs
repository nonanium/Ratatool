use std::{collections::BTreeMap, error::Error, fmt};

use crate::terminal::TerminalProfile;

use super::{
    ColorFallback, ResolvedTheme, ThemeFamily, ThemeId, ThemeResolveError, resolve_theme,
    resolve_theme_with_fallback,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThemeError {
    UnknownTheme(ThemeId),
    UnsupportedColor(ThemeResolveError),
}

impl fmt::Display for ThemeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownTheme(id) => write!(formatter, "unknown theme: {id}"),
            Self::UnsupportedColor(error) => error.fmt(formatter),
        }
    }
}

impl Error for ThemeError {}

impl From<ThemeResolveError> for ThemeError {
    fn from(error: ThemeResolveError) -> Self {
        Self::UnsupportedColor(error)
    }
}

#[derive(Clone, Debug, Default)]
pub struct ThemeRegistry {
    themes: BTreeMap<ThemeId, ThemeFamily>,
}

impl ThemeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a family, returning the previous family with the same ID.
    pub fn register(&mut self, theme: ThemeFamily) -> Option<ThemeFamily> {
        self.themes.insert(theme.id.clone(), theme)
    }

    pub fn get(&self, id: &ThemeId) -> Option<&ThemeFamily> {
        self.themes.get(id)
    }

    pub fn contains(&self, id: &ThemeId) -> bool {
        self.themes.contains_key(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&ThemeId, &ThemeFamily)> {
        self.themes.iter()
    }

    pub fn ids(&self) -> impl Iterator<Item = &ThemeId> {
        self.themes.keys()
    }

    pub fn resolve(
        &self,
        id: &ThemeId,
        profile: TerminalProfile,
    ) -> Result<ResolvedTheme, ThemeError> {
        let family = self
            .get(id)
            .ok_or_else(|| ThemeError::UnknownTheme(id.clone()))?;
        Ok(resolve_theme(family, profile))
    }

    pub fn resolve_with_fallback(
        &self,
        id: &ThemeId,
        profile: TerminalProfile,
        fallback: ColorFallback,
    ) -> Result<ResolvedTheme, ThemeError> {
        let family = self
            .get(id)
            .ok_or_else(|| ThemeError::UnknownTheme(id.clone()))?;
        resolve_theme_with_fallback(family, profile, fallback).map_err(ThemeError::from)
    }
}

/// Runtime theme selection that retains the detected terminal profile.
#[derive(Clone, Debug)]
pub struct ThemeState {
    profile: TerminalProfile,
    selected: ThemeId,
    resolved: ResolvedTheme,
}

impl ThemeState {
    pub fn new(
        selected: ThemeId,
        profile: TerminalProfile,
        registry: &ThemeRegistry,
    ) -> Result<Self, ThemeError> {
        let resolved = registry.resolve(&selected, profile)?;
        Ok(Self {
            profile,
            selected,
            resolved,
        })
    }

    pub fn select_theme(
        &mut self,
        id: ThemeId,
        registry: &ThemeRegistry,
    ) -> Result<(), ThemeError> {
        let resolved = registry.resolve(&id, self.profile)?;
        self.selected = id;
        self.resolved = resolved;
        Ok(())
    }

    pub fn set_theme(&mut self, id: ThemeId, registry: &ThemeRegistry) -> Result<(), ThemeError> {
        self.select_theme(id, registry)
    }

    pub fn set_profile(
        &mut self,
        profile: TerminalProfile,
        registry: &ThemeRegistry,
    ) -> Result<(), ThemeError> {
        let resolved = registry.resolve(&self.selected, profile)?;
        self.profile = profile;
        self.resolved = resolved;
        Ok(())
    }

    pub fn re_resolve(&mut self, registry: &ThemeRegistry) -> Result<(), ThemeError> {
        self.resolved = registry.resolve(&self.selected, self.profile)?;
        Ok(())
    }

    pub const fn profile(&self) -> TerminalProfile {
        self.profile
    }

    pub fn selected(&self) -> &ThemeId {
        &self.selected
    }

    pub const fn resolved(&self) -> &ResolvedTheme {
        &self.resolved
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        Charset, ColorDepth, ModifierSupport, ThemeVariant,
        theme::{Symbols, ThemeFamily},
    };

    use super::*;

    fn family(id: &str) -> ThemeFamily {
        ThemeFamily::new(
            ThemeId::new(id),
            ThemeVariant::default(),
            ThemeVariant::default(),
            ThemeVariant::default(),
            ThemeVariant::default(),
        )
    }

    fn profile() -> TerminalProfile {
        TerminalProfile {
            color_depth: ColorDepth::None,
            charset: Charset::Ascii,
            modifiers: ModifierSupport::NONE,
        }
    }

    #[test]
    fn resolves_registered_theme() {
        let mut registry = ThemeRegistry::new();
        registry.register(family("first"));
        let resolved = registry.resolve(&ThemeId::new("first"), profile()).unwrap();
        assert_eq!(resolved.symbols, Symbols::ascii());
    }

    #[test]
    fn reports_unknown_theme() {
        let error = ThemeRegistry::new()
            .resolve(&ThemeId::new("missing"), profile())
            .unwrap_err();
        assert_eq!(error, ThemeError::UnknownTheme(ThemeId::new("missing")));
    }

    #[test]
    fn switching_theme_preserves_profile() {
        let mut registry = ThemeRegistry::new();
        registry.register(family("first"));
        registry.register(family("second"));
        let mut state = ThemeState::new(ThemeId::new("first"), profile(), &registry).unwrap();

        state
            .select_theme(ThemeId::new("second"), &registry)
            .unwrap();

        assert_eq!(state.profile(), profile());
        assert_eq!(state.selected(), &ThemeId::new("second"));
        assert_eq!(state.resolved().symbols, Symbols::ascii());
    }

    #[test]
    fn profile_changes_are_resolved_atomically() {
        let mut registry = ThemeRegistry::new();
        registry.register(family("first"));
        let mut state = ThemeState::new(ThemeId::new("first"), profile(), &registry).unwrap();
        state
            .set_profile(TerminalProfile::truecolor_unicode(), &registry)
            .unwrap();
        assert_eq!(state.profile(), TerminalProfile::truecolor_unicode());
        assert_eq!(state.resolved().symbols, Symbols::unicode());
        assert!(registry.contains(state.selected()));
        assert_eq!(registry.ids().count(), 1);
    }
}
