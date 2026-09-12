#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BorderSymbols {
    pub horizontal: &'static str,
    pub vertical: &'static str,
    pub top_left: &'static str,
    pub top_right: &'static str,
    pub bottom_left: &'static str,
    pub bottom_right: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScrollSymbols {
    pub up: &'static str,
    pub down: &'static str,
    pub track: &'static str,
    pub thumb: &'static str,
}

/// Presentation glyphs selected once during theme resolution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Symbols {
    pub focus_marker: &'static str,
    pub checkbox_checked: &'static str,
    pub checkbox_unchecked: &'static str,
    pub radio_checked: &'static str,
    pub radio_unchecked: &'static str,
    pub status_info: &'static str,
    pub status_success: &'static str,
    pub status_warning: &'static str,
    pub status_error: &'static str,
    pub password_mask: &'static str,
    pub progress_full: &'static str,
    pub progress_empty: &'static str,
    pub borders: BorderSymbols,
    pub scroll: ScrollSymbols,
}

impl Symbols {
    pub const fn unicode() -> Self {
        Self {
            focus_marker: "›",
            checkbox_checked: "☑",
            checkbox_unchecked: "☐",
            radio_checked: "◉",
            radio_unchecked: "○",
            status_info: "ℹ",
            status_success: "✓",
            status_warning: "⚠",
            status_error: "✗",
            password_mask: "•",
            progress_full: "█",
            progress_empty: "░",
            borders: BorderSymbols {
                horizontal: "─",
                vertical: "│",
                top_left: "┌",
                top_right: "┐",
                bottom_left: "└",
                bottom_right: "┘",
            },
            scroll: ScrollSymbols {
                up: "↑",
                down: "↓",
                track: "│",
                thumb: "█",
            },
        }
    }

    pub const fn ascii() -> Self {
        Self {
            focus_marker: ">",
            checkbox_checked: "[x]",
            checkbox_unchecked: "[ ]",
            radio_checked: "(x)",
            radio_unchecked: "( )",
            status_info: "i",
            status_success: "+",
            status_warning: "!",
            status_error: "x",
            password_mask: "*",
            progress_full: "#",
            progress_empty: ".",
            borders: BorderSymbols {
                horizontal: "-",
                vertical: "|",
                top_left: "+",
                top_right: "+",
                bottom_left: "+",
                bottom_right: "+",
            },
            scroll: ScrollSymbols {
                up: "^",
                down: "v",
                track: "|",
                thumb: "#",
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SymbolFamily {
    pub unicode: Symbols,
    pub ascii: Symbols,
}

impl Default for SymbolFamily {
    fn default() -> Self {
        Self {
            unicode: Symbols::unicode(),
            ascii: Symbols::ascii(),
        }
    }
}
