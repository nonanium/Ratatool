use std::io::IsTerminal;

use super::{
    CapabilityPolicy, Charset, ColorDepth, ModifierSupport, TerminalOptions, TerminalProfile,
};

/// Process and environment facts consumed by capability detection.
///
/// Keeping these facts in a value makes detection deterministic in tests and
/// lets applications supply information from a source other than process-wide
/// environment variables.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DetectionInput {
    pub stdin_is_terminal: bool,
    pub stdout_is_terminal: bool,
    pub term: Option<String>,
    pub colorterm: Option<String>,
    pub no_color: bool,
    pub lang: Option<String>,
    pub lc_all: Option<String>,
    pub lc_ctype: Option<String>,
}

impl DetectionInput {
    /// Collects terminal and locale information from the current process.
    pub fn current() -> Self {
        Self {
            stdin_is_terminal: std::io::stdin().is_terminal(),
            stdout_is_terminal: std::io::stdout().is_terminal(),
            term: std::env::var("TERM").ok(),
            colorterm: std::env::var("COLORTERM").ok(),
            no_color: std::env::var_os("NO_COLOR").is_some(),
            lang: std::env::var("LANG").ok(),
            lc_all: std::env::var("LC_ALL").ok(),
            lc_ctype: std::env::var("LC_CTYPE").ok(),
        }
    }
}

/// Detects a profile from explicit, testable process facts and application policy.
pub fn detect_profile(input: &DetectionInput, options: &TerminalOptions) -> TerminalProfile {
    let interactive = input.stdin_is_terminal && input.stdout_is_terminal;
    let dumb = input
        .term
        .as_deref()
        .is_some_and(|term| term.eq_ignore_ascii_case("dumb"));

    let detected_color_depth = match options.color {
        CapabilityPolicy::Disable => ColorDepth::None,
        CapabilityPolicy::Force => detected_color_depth(input).unwrap_or(ColorDepth::Ansi16),
        CapabilityPolicy::Auto if !interactive || dumb || input.no_color => ColorDepth::None,
        CapabilityPolicy::Auto => detected_color_depth(input).unwrap_or(ColorDepth::Ansi16),
    };
    let color_depth = options
        .overrides
        .color_depth
        .unwrap_or(detected_color_depth);

    let detected_charset = match options.unicode {
        CapabilityPolicy::Disable => Charset::Ascii,
        CapabilityPolicy::Force => Charset::Unicode,
        CapabilityPolicy::Auto if !interactive || dumb => Charset::Ascii,
        CapabilityPolicy::Auto if locale_supports_unicode(input) => Charset::Unicode,
        CapabilityPolicy::Auto => Charset::Ascii,
    };
    let charset = options.overrides.charset.unwrap_or(detected_charset);

    // Crossterm cannot query individual text attributes. COMMON is an assumed
    // baseline for normal interactive terminals and can be overridden exactly.
    let modifiers = options.overrides.modifiers.unwrap_or({
        if interactive && !dumb {
            ModifierSupport::COMMON
        } else {
            ModifierSupport::NONE
        }
    });

    TerminalProfile {
        color_depth,
        charset,
        modifiers,
    }
}

/// Detects capabilities for the current process.
pub fn detect_current(options: &TerminalOptions) -> TerminalProfile {
    detect_profile(&DetectionInput::current(), options)
}

fn detected_color_depth(input: &DetectionInput) -> Option<ColorDepth> {
    let colorterm = input.colorterm.as_deref().unwrap_or_default();
    if colorterm.eq_ignore_ascii_case("truecolor") || colorterm.eq_ignore_ascii_case("24bit") {
        return Some(ColorDepth::TrueColor);
    }

    let term = input.term.as_deref()?;
    if term.to_ascii_lowercase().contains("256color") {
        Some(ColorDepth::Ansi256)
    } else if term.eq_ignore_ascii_case("dumb") {
        None
    } else {
        Some(ColorDepth::Ansi16)
    }
}

fn locale_supports_unicode(input: &DetectionInput) -> bool {
    let locale = input
        .lc_all
        .as_deref()
        .filter(|value| !value.is_empty())
        .or_else(|| input.lc_ctype.as_deref().filter(|value| !value.is_empty()))
        .or_else(|| input.lang.as_deref().filter(|value| !value.is_empty()));

    locale.is_some_and(|value| {
        let normalized = value.to_ascii_lowercase().replace('-', "");
        normalized.contains("utf8")
    })
}

#[cfg(test)]
mod tests {
    use crate::terminal::TerminalOverrides;

    use super::*;

    fn interactive() -> DetectionInput {
        DetectionInput {
            stdin_is_terminal: true,
            stdout_is_terminal: true,
            term: Some("xterm-256color".into()),
            colorterm: Some("truecolor".into()),
            lang: Some("en_US.UTF-8".into()),
            ..DetectionInput::default()
        }
    }

    #[test]
    fn detects_truecolor_and_unicode() {
        let profile = detect_profile(&interactive(), &TerminalOptions::default());
        assert_eq!(profile.color_depth, ColorDepth::TrueColor);
        assert_eq!(profile.charset, Charset::Unicode);
        assert_eq!(profile.modifiers, ModifierSupport::COMMON);
    }

    #[test]
    fn no_color_only_affects_auto_color() {
        let input = DetectionInput {
            no_color: true,
            ..interactive()
        };
        let automatic = detect_profile(&input, &TerminalOptions::default());
        assert_eq!(automatic.color_depth, ColorDepth::None);

        let forced = detect_profile(
            &input,
            &TerminalOptions {
                color: CapabilityPolicy::Force,
                unicode: CapabilityPolicy::Auto,
                ..TerminalOptions::default()
            },
        );
        assert_eq!(forced.color_depth, ColorDepth::TrueColor);
    }

    #[test]
    fn explicit_disable_wins_over_detected_capabilities() {
        let profile = detect_profile(
            &interactive(),
            &TerminalOptions {
                color: CapabilityPolicy::Disable,
                unicode: CapabilityPolicy::Disable,
                ..TerminalOptions::default()
            },
        );
        assert_eq!(profile.color_depth, ColorDepth::None);
        assert_eq!(profile.charset, Charset::Ascii);
    }

    #[test]
    fn dumb_terminal_is_plain_unless_features_are_forced() {
        let input = DetectionInput {
            term: Some("dumb".into()),
            ..interactive()
        };
        let automatic = detect_profile(&input, &TerminalOptions::default());
        assert_eq!(automatic.color_depth, ColorDepth::None);
        assert_eq!(automatic.charset, Charset::Ascii);
        assert_eq!(automatic.modifiers, ModifierSupport::NONE);

        let forced = detect_profile(
            &input,
            &TerminalOptions {
                color: CapabilityPolicy::Force,
                unicode: CapabilityPolicy::Force,
                ..TerminalOptions::default()
            },
        );
        assert_eq!(forced.color_depth, ColorDepth::TrueColor);
        assert_eq!(forced.charset, Charset::Unicode);
    }

    #[test]
    fn redirected_process_uses_conservative_defaults() {
        let input = DetectionInput {
            term: Some("xterm-256color".into()),
            lang: Some("en_US.UTF-8".into()),
            ..DetectionInput::default()
        };
        let profile = detect_profile(&input, &TerminalOptions::default());
        assert_eq!(profile.color_depth, ColorDepth::None);
        assert_eq!(profile.charset, Charset::Ascii);
        assert_eq!(profile.modifiers, ModifierSupport::NONE);
    }

    #[test]
    fn locale_precedence_controls_unicode_detection() {
        let input = DetectionInput {
            lc_all: Some("C".into()),
            lc_ctype: Some("en_US.UTF-8".into()),
            ..interactive()
        };
        let profile = detect_profile(&input, &TerminalOptions::default());
        assert_eq!(profile.charset, Charset::Ascii);
    }

    #[test]
    fn explicit_modifier_override_wins_over_interactive_baseline() {
        let profile = detect_profile(
            &interactive(),
            &TerminalOptions {
                overrides: TerminalOverrides {
                    modifiers: Some(ModifierSupport::NONE),
                    ..TerminalOverrides::default()
                },
                ..TerminalOptions::default()
            },
        );
        assert_eq!(profile.modifiers, ModifierSupport::NONE);
    }

    #[test]
    fn exact_overrides_replace_detected_color_and_charset() {
        let profile = detect_profile(
            &interactive(),
            &TerminalOptions {
                overrides: TerminalOverrides {
                    color_depth: Some(ColorDepth::Ansi16),
                    charset: Some(Charset::Ascii),
                    modifiers: None,
                },
                ..TerminalOptions::default()
            },
        );
        assert_eq!(profile.color_depth, ColorDepth::Ansi16);
        assert_eq!(profile.charset, Charset::Ascii);
    }
}
