//! Adaptive, themeable terminal UI building blocks for Ratatui.
//!
//! Ratatool keeps terminal capabilities, visual themes, and viewport layout as
//! separate concerns. Applications can keep using Ratatui types directly while
//! sharing the policy needed to render safely across different terminals.

pub mod framework;
pub mod layout;
pub mod terminal;
pub mod theme;
pub mod widgets;

#[cfg(feature = "tokio")]
pub use framework::AsyncEventSource;
pub use framework::{
    Direction, FocusEntry, FocusError, FocusGraph, FocusManager, HitMap, HitRegion, Interaction,
    RedrawState, Runtime, Screen, ScreenPresentation, ScreenStack, SessionOptions, TerminalSession,
    UiEvent, UiFrame,
};

pub use terminal::{
    CapabilityPolicy, Charset, ColorDepth, DetectionInput, ModifierSupport, RenderClass,
    TerminalOptions, TerminalOverrides, TerminalProfile, detect_current, detect_profile,
};
pub use theme::{
    BorderSymbols, BorderTheme, ButtonTheme, ColorFallback, InputTheme, PanelPresentation,
    ResolvedTheme, ScrollSymbols, SelectionTheme, StatusTheme, SurfaceTheme, SymbolFamily, Symbols,
    TextTheme, ThemeError, ThemeFamily, ThemeId, ThemeMetadata, ThemeMetrics, ThemeRegistry,
    ThemeResolveError, ThemeState, ThemeVariant, resolve_theme, resolve_theme_with_fallback,
};
pub use widgets::{
    Button, ButtonKind, CheckboxChange, CheckboxGroup, CheckboxGroupError, CheckboxItem,
    ChoiceKind, ChoiceVisualState, Dialog, DialogButton, DisabledFocusPolicy,
    DisabledSelectionPolicy, MenuError, MenuItem, MenuRenderOptions, MenuResult, MenuState,
    Notification, NotificationLevel, NotificationOverlay, NotificationResult, Padding, Panel,
    PanelKind, RadioChange, RadioGroup, RadioGroupError, RadioItem, ScrollOptions, ScrollResult,
    ScrollState, TextInputConfig, TextInputResult, TextInputState, button_minimum_width,
    horizontal_button_widths, render_button, render_choice_line, render_dialog, render_menu,
    render_menu_with_options, render_notifications, render_panel, render_scroll, render_scrollbar,
    render_text_input,
};
