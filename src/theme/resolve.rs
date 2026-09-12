use std::{error::Error, fmt};

use ratatui::style::{Color, Modifier, Style};

use crate::terminal::{Charset, ColorDepth, RenderClass, TerminalProfile};

use super::{ResolvedTheme, ThemeFamily, ThemeVariant};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ColorFallback {
    Reject,
    #[default]
    Quantize,
    Reset,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThemeResolveError {
    pub color: Color,
    pub depth: ColorDepth,
}

impl fmt::Display for ThemeResolveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "color {:?} cannot be represented at {:?} depth",
            self.color, self.depth
        )
    }
}

impl Error for ThemeResolveError {}

/// Selects a family variant and strips styling unsupported by the terminal.
pub fn resolve_theme(family: &ThemeFamily, profile: TerminalProfile) -> ResolvedTheme {
    // Quantization is infallible and is the convenient adaptive-rendering default.
    resolve_theme_with_fallback(family, profile, ColorFallback::Quantize)
        .expect("ColorFallback::Quantize cannot reject a color")
}

pub fn resolve_theme_with_fallback(
    family: &ThemeFamily,
    profile: TerminalProfile,
    fallback: ColorFallback,
) -> Result<ResolvedTheme, ThemeResolveError> {
    let variant = match profile.render_class() {
        RenderClass::FullColor => &family.full_color,
        RenderClass::Color => &family.color,
        RenderClass::StyledMonochrome => &family.styled_monochrome,
        RenderClass::Plain => &family.plain,
    };
    let normalized = normalize_variant(variant, profile, fallback)?;

    Ok(ResolvedTheme {
        text: normalized.text,
        borders: normalized.borders,
        surfaces: normalized.surfaces,
        buttons: normalized.buttons,
        status: normalized.status,
        input: normalized.input,
        selection: normalized.selection,
        metrics: normalized.metrics,
        symbols: match profile.charset {
            Charset::Ascii => family.symbols.ascii,
            Charset::Unicode => family.symbols.unicode,
        },
    })
}

fn normalize_variant(
    variant: &ThemeVariant,
    profile: TerminalProfile,
    fallback: ColorFallback,
) -> Result<ThemeVariant, ThemeResolveError> {
    let mut resolved = variant.clone();
    let mut error = None;
    visit_styles(&mut resolved, |style| {
        if error.is_none() {
            error = normalize_style(style, profile, fallback).err();
        }
    });
    error.map_or(Ok(resolved), Err)
}

fn visit_styles(theme: &mut ThemeVariant, mut visit: impl FnMut(&mut Style)) {
    visit(&mut theme.text.normal);
    visit(&mut theme.text.muted);
    visit(&mut theme.text.emphasis);
    visit(&mut theme.text.disabled);
    visit(&mut theme.text.heading);
    visit(&mut theme.text.link);
    visit(&mut theme.text.code);
    visit(&mut theme.borders.normal);
    visit(&mut theme.borders.focused);
    visit(&mut theme.borders.disabled);
    visit(&mut theme.surfaces.background);
    visit(&mut theme.surfaces.panel);
    visit(&mut theme.surfaces.panel_focused);
    visit(&mut theme.surfaces.panel_header);
    visit(&mut theme.surfaces.raised);
    visit(&mut theme.surfaces.toolbar);
    visit(&mut theme.surfaces.footer);
    visit(&mut theme.surfaces.overlay);
    visit(&mut theme.buttons.primary);
    visit(&mut theme.buttons.primary_focused);
    visit(&mut theme.buttons.secondary);
    visit(&mut theme.buttons.secondary_focused);
    visit(&mut theme.buttons.cancel);
    visit(&mut theme.buttons.cancel_focused);
    visit(&mut theme.buttons.destructive);
    visit(&mut theme.buttons.destructive_focused);
    visit(&mut theme.buttons.disabled);
    visit(&mut theme.status.info);
    visit(&mut theme.status.success);
    visit(&mut theme.status.warning);
    visit(&mut theme.status.error);
    visit(&mut theme.input.normal);
    visit(&mut theme.input.focused);
    visit(&mut theme.input.placeholder);
    visit(&mut theme.input.selection);
    visit(&mut theme.selection.normal);
    visit(&mut theme.selection.selected);
    visit(&mut theme.selection.focused);
    visit(&mut theme.selection.disabled);
}

fn normalize_style(
    style: &mut Style,
    profile: TerminalProfile,
    fallback: ColorFallback,
) -> Result<(), ThemeResolveError> {
    style.fg = normalize_color(style.fg, profile.color_depth, fallback)?;
    style.bg = normalize_color(style.bg, profile.color_depth, fallback)?;
    style.underline_color = normalize_color(style.underline_color, profile.color_depth, fallback)?;

    remove_unsupported_modifier(style, Modifier::BOLD, profile.modifiers.bold);
    remove_unsupported_modifier(style, Modifier::DIM, profile.modifiers.dim);
    remove_unsupported_modifier(style, Modifier::ITALIC, profile.modifiers.italic);
    remove_unsupported_modifier(style, Modifier::UNDERLINED, profile.modifiers.underline);
    remove_unsupported_modifier(style, Modifier::REVERSED, profile.modifiers.reverse);

    // These attributes are deliberately outside the initial capability model.
    for modifier in [
        Modifier::SLOW_BLINK,
        Modifier::RAPID_BLINK,
        Modifier::HIDDEN,
        Modifier::CROSSED_OUT,
    ] {
        style.add_modifier.remove(modifier);
        style.sub_modifier.remove(modifier);
    }
    Ok(())
}

fn remove_unsupported_modifier(style: &mut Style, modifier: Modifier, supported: bool) {
    if !supported {
        style.add_modifier.remove(modifier);
        style.sub_modifier.remove(modifier);
    }
}

fn normalize_color(
    color: Option<Color>,
    depth: ColorDepth,
    fallback: ColorFallback,
) -> Result<Option<Color>, ThemeResolveError> {
    match (color, depth) {
        (_, ColorDepth::None) => Ok(None),
        (Some(color @ Color::Rgb(_, _, _)), ColorDepth::Ansi256) => {
            degrade_color(color, depth, fallback, quantize_ansi256)
        }
        (Some(color @ Color::Rgb(_, _, _)), ColorDepth::Ansi16) => {
            degrade_color(color, depth, fallback, quantize_ansi16)
        }
        (Some(color @ Color::Indexed(index)), ColorDepth::Ansi16) if index > 15 => {
            degrade_color(color, depth, fallback, quantize_ansi16)
        }
        (color, _) => Ok(color),
    }
}

fn degrade_color(
    color: Color,
    depth: ColorDepth,
    fallback: ColorFallback,
    quantize: impl FnOnce(Color) -> Color,
) -> Result<Option<Color>, ThemeResolveError> {
    match fallback {
        ColorFallback::Reject => Err(ThemeResolveError { color, depth }),
        ColorFallback::Quantize => Ok(Some(quantize(color))),
        ColorFallback::Reset => Ok(Some(Color::Reset)),
    }
}

fn quantize_ansi256(color: Color) -> Color {
    let (red, green, blue) = color_rgb(color);
    let cube = |channel: u8| ((u16::from(channel) * 5 + 127) / 255) as u8;
    Color::Indexed(16 + 36 * cube(red) + 6 * cube(green) + cube(blue))
}

fn quantize_ansi16(color: Color) -> Color {
    const PALETTE: [(u8, u8, u8); 16] = [
        (0, 0, 0),
        (128, 0, 0),
        (0, 128, 0),
        (128, 128, 0),
        (0, 0, 128),
        (128, 0, 128),
        (0, 128, 128),
        (192, 192, 192),
        (128, 128, 128),
        (255, 0, 0),
        (0, 255, 0),
        (255, 255, 0),
        (0, 0, 255),
        (255, 0, 255),
        (0, 255, 255),
        (255, 255, 255),
    ];
    let (red, green, blue) = color_rgb(color);
    let index = PALETTE
        .iter()
        .enumerate()
        .min_by_key(|(_, color)| color_distance((red, green, blue), **color))
        .map_or(0, |(index, _)| index);
    Color::Indexed(index as u8)
}

fn color_rgb(color: Color) -> (u8, u8, u8) {
    match color {
        Color::Rgb(red, green, blue) => (red, green, blue),
        Color::Indexed(index) if index < 16 => {
            const ANSI: [(u8, u8, u8); 16] = [
                (0, 0, 0),
                (128, 0, 0),
                (0, 128, 0),
                (128, 128, 0),
                (0, 0, 128),
                (128, 0, 128),
                (0, 128, 128),
                (192, 192, 192),
                (128, 128, 128),
                (255, 0, 0),
                (0, 255, 0),
                (255, 255, 0),
                (0, 0, 255),
                (255, 0, 255),
                (0, 255, 255),
                (255, 255, 255),
            ];
            ANSI[usize::from(index)]
        }
        Color::Indexed(index @ 16..=231) => {
            let index = index - 16;
            let scale = |value: u8| if value == 0 { 0 } else { 55 + value * 40 };
            (scale(index / 36), scale(index / 6 % 6), scale(index % 6))
        }
        Color::Indexed(index) => {
            let value = 8 + (index - 232) * 10;
            (value, value, value)
        }
        _ => (0, 0, 0),
    }
}

fn color_distance(left: (u8, u8, u8), right: (u8, u8, u8)) -> u32 {
    let channel = |a: u8, b: u8| {
        let difference = i32::from(a) - i32::from(b);
        (difference * difference) as u32
    };
    channel(left.0, right.0) + channel(left.1, right.1) + channel(left.2, right.2)
}

#[cfg(test)]
mod tests {
    use ratatui::style::{Color, Modifier, Style};

    use crate::terminal::{Charset, ColorDepth, ModifierSupport, TerminalProfile};

    use super::*;
    use crate::theme::{Symbols, ThemeId, ThemeVariant};

    fn family() -> ThemeFamily {
        let variant = |color| ThemeVariant {
            text: crate::theme::TextTheme {
                normal: Style::default()
                    .fg(color)
                    .bg(Color::Rgb(1, 2, 3))
                    .add_modifier(Modifier::BOLD | Modifier::REVERSED),
                ..Default::default()
            },
            ..Default::default()
        };
        ThemeFamily::new(
            ThemeId::new("test"),
            variant(Color::Rgb(10, 20, 30)),
            variant(Color::Indexed(200)),
            variant(Color::Red),
            variant(Color::Green),
        )
    }

    #[test]
    fn chooses_variant_and_charset_from_profile() {
        let resolved = resolve_theme(
            &family(),
            TerminalProfile {
                color_depth: ColorDepth::TrueColor,
                charset: Charset::Ascii,
                modifiers: ModifierSupport::COMMON,
            },
        );
        assert_eq!(resolved.text.normal.fg, Some(Color::Rgb(10, 20, 30)));
        assert_eq!(resolved.symbols, Symbols::ascii());
    }

    #[test]
    fn plain_profile_removes_colors_and_modifiers() {
        let resolved = resolve_theme(
            &family(),
            TerminalProfile {
                color_depth: ColorDepth::None,
                charset: Charset::Unicode,
                modifiers: ModifierSupport::NONE,
            },
        );
        assert_eq!(resolved.text.normal.fg, None);
        assert_eq!(resolved.text.normal.bg, None);
        assert!(resolved.text.normal.add_modifier.is_empty());
        assert_eq!(resolved.symbols, Symbols::unicode());
    }

    #[test]
    fn ansi16_profile_quantizes_extended_palette_entries() {
        let resolved = resolve_theme(
            &family(),
            TerminalProfile {
                color_depth: ColorDepth::Ansi16,
                charset: Charset::Unicode,
                modifiers: ModifierSupport::COMMON,
            },
        );
        assert!(matches!(
            resolved.text.normal.fg,
            Some(Color::Indexed(0..=15))
        ));
        assert!(matches!(
            resolved.text.normal.bg,
            Some(Color::Indexed(0..=15))
        ));
    }

    #[test]
    fn strict_fallback_rejects_unrepresentable_colors() {
        let error = resolve_theme_with_fallback(
            &family(),
            TerminalProfile::ansi16_unicode(),
            ColorFallback::Reject,
        )
        .unwrap_err();
        assert_eq!(error.depth, ColorDepth::Ansi16);
    }

    #[test]
    fn removes_only_unsupported_modeled_modifiers() {
        let resolved = resolve_theme(
            &family(),
            TerminalProfile {
                color_depth: ColorDepth::None,
                charset: Charset::Unicode,
                modifiers: ModifierSupport {
                    reverse: true,
                    ..ModifierSupport::NONE
                },
            },
        );
        assert!(!resolved.text.normal.add_modifier.contains(Modifier::BOLD));
        assert!(
            resolved
                .text
                .normal
                .add_modifier
                .contains(Modifier::REVERSED)
        );
    }
}
