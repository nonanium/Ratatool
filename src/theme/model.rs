use ratatui::style::Style;

use super::Symbols;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TextTheme {
    pub normal: Style,
    pub muted: Style,
    pub emphasis: Style,
    pub disabled: Style,
    pub heading: Style,
    pub link: Style,
    pub code: Style,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BorderTheme {
    pub normal: Style,
    pub focused: Style,
    pub disabled: Style,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SurfaceTheme {
    pub background: Style,
    pub panel: Style,
    pub panel_focused: Style,
    pub panel_header: Style,
    pub raised: Style,
    pub toolbar: Style,
    pub footer: Style,
    pub overlay: Style,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ButtonTheme {
    pub primary: Style,
    pub primary_focused: Style,
    pub secondary: Style,
    pub secondary_focused: Style,
    pub cancel: Style,
    pub cancel_focused: Style,
    pub destructive: Style,
    pub destructive_focused: Style,
    pub disabled: Style,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PanelPresentation {
    #[default]
    Bordered,
    Surface,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ThemeMetrics {
    pub panel_presentation: PanelPresentation,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StatusTheme {
    pub info: Style,
    pub success: Style,
    pub warning: Style,
    pub error: Style,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InputTheme {
    pub normal: Style,
    pub focused: Style,
    pub placeholder: Style,
    pub selection: Style,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SelectionTheme {
    pub normal: Style,
    /// Selected state when the owning control does not have keyboard focus.
    pub selected: Style,
    pub focused: Style,
    pub disabled: Style,
}

/// A theme representation authored for one broad rendering class.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ThemeVariant {
    pub text: TextTheme,
    pub borders: BorderTheme,
    pub surfaces: SurfaceTheme,
    pub buttons: ButtonTheme,
    pub status: StatusTheme,
    pub input: InputTheme,
    pub selection: SelectionTheme,
    pub metrics: ThemeMetrics,
}

/// Semantic styles and symbols normalized for a concrete terminal profile.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedTheme {
    pub text: TextTheme,
    pub borders: BorderTheme,
    pub surfaces: SurfaceTheme,
    pub buttons: ButtonTheme,
    pub status: StatusTheme,
    pub input: InputTheme,
    pub selection: SelectionTheme,
    pub metrics: ThemeMetrics,
    pub symbols: Symbols,
}
