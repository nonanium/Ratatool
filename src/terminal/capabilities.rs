use std::fmt;

/// The color palette a terminal can display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorDepth {
    None,
    Ansi16,
    Ansi256,
    TrueColor,
}

/// The character repertoire safe to use for presentation glyphs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Charset {
    Ascii,
    Unicode,
}

/// Text attributes supported by the terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModifierSupport {
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub underline: bool,
    pub reverse: bool,
}

impl ModifierSupport {
    pub const NONE: Self = Self {
        bold: false,
        dim: false,
        italic: false,
        underline: false,
        reverse: false,
    };

    pub const COMMON: Self = Self {
        bold: true,
        dim: true,
        italic: true,
        underline: true,
        reverse: true,
    };

    pub const fn any(self) -> bool {
        self.bold || self.dim || self.italic || self.underline || self.reverse
    }
}

/// The rendering capabilities available to the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalProfile {
    pub color_depth: ColorDepth,
    pub charset: Charset,
    pub modifiers: ModifierSupport,
}

impl TerminalProfile {
    /// Reduces detailed capabilities to the visual strategy used by themes.
    pub fn render_class(self) -> RenderClass {
        match self.color_depth {
            ColorDepth::TrueColor => RenderClass::FullColor,
            ColorDepth::Ansi256 | ColorDepth::Ansi16 => RenderClass::Color,
            ColorDepth::None if self.modifiers.any() => RenderClass::StyledMonochrome,
            ColorDepth::None => RenderClass::Plain,
        }
    }

    pub const fn plain_ascii() -> Self {
        Self::new(ColorDepth::None, Charset::Ascii, ModifierSupport::NONE)
    }

    pub const fn styled_ascii() -> Self {
        Self::new(ColorDepth::None, Charset::Ascii, ModifierSupport::COMMON)
    }

    pub const fn ansi16_unicode() -> Self {
        Self::new(
            ColorDepth::Ansi16,
            Charset::Unicode,
            ModifierSupport::COMMON,
        )
    }

    pub const fn ansi256_unicode() -> Self {
        Self::new(
            ColorDepth::Ansi256,
            Charset::Unicode,
            ModifierSupport::COMMON,
        )
    }

    pub const fn truecolor_unicode() -> Self {
        Self::new(
            ColorDepth::TrueColor,
            Charset::Unicode,
            ModifierSupport::COMMON,
        )
    }

    pub const fn new(
        color_depth: ColorDepth,
        charset: Charset,
        modifiers: ModifierSupport,
    ) -> Self {
        Self {
            color_depth,
            charset,
            modifiers,
        }
    }
}

impl fmt::Display for TerminalProfile {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "color: {:?}, charset: {:?}, modifiers: bold={}, dim={}, italic={}, underline={}, reverse={}, render class: {:?}",
            self.color_depth,
            self.charset,
            self.modifiers.bold,
            self.modifiers.dim,
            self.modifiers.italic,
            self.modifiers.underline,
            self.modifiers.reverse,
            self.render_class(),
        )
    }
}

/// Broad presentation strategy selected from terminal capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderClass {
    FullColor,
    Color,
    StyledMonochrome,
    Plain,
}

/// Whether capability detection should decide, enable, or disable a feature.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CapabilityPolicy {
    #[default]
    Auto,
    Force,
    Disable,
}

/// Exact application overrides applied after terminal detection policy.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TerminalOverrides {
    pub color_depth: Option<ColorDepth>,
    pub charset: Option<Charset>,
    pub modifiers: Option<ModifierSupport>,
}

/// Detection policies plus exact overrides for testing and user preferences.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TerminalOptions {
    pub color: CapabilityPolicy,
    pub unicode: CapabilityPolicy,
    pub overrides: TerminalOverrides,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(color_depth: ColorDepth, modifiers: ModifierSupport) -> TerminalProfile {
        TerminalProfile {
            color_depth,
            charset: Charset::Unicode,
            modifiers,
        }
    }

    #[test]
    fn classifies_each_color_depth() {
        assert_eq!(
            profile(ColorDepth::TrueColor, ModifierSupport::COMMON).render_class(),
            RenderClass::FullColor
        );
        assert_eq!(
            profile(ColorDepth::Ansi256, ModifierSupport::COMMON).render_class(),
            RenderClass::Color
        );
        assert_eq!(
            profile(ColorDepth::Ansi16, ModifierSupport::COMMON).render_class(),
            RenderClass::Color
        );
    }

    #[test]
    fn distinguishes_styled_and_plain_monochrome() {
        assert_eq!(
            profile(ColorDepth::None, ModifierSupport::COMMON).render_class(),
            RenderClass::StyledMonochrome
        );
        assert_eq!(
            profile(ColorDepth::None, ModifierSupport::NONE).render_class(),
            RenderClass::Plain
        );
        assert_eq!(
            profile(
                ColorDepth::None,
                ModifierSupport {
                    italic: true,
                    ..ModifierSupport::NONE
                }
            )
            .render_class(),
            RenderClass::StyledMonochrome
        );
    }
}
