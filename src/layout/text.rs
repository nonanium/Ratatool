use unicode_width::UnicodeWidthStr;

/// Counts the terminal rows needed to display newline-separated, wrapped text.
pub fn hard_wrapped_line_count(text: &str, width: u16) -> u16 {
    if width == 0 {
        return 0;
    }
    let width = usize::from(width);
    text.split('\n')
        .map(|line| UnicodeWidthStr::width(line).max(1).div_ceil(width))
        .sum::<usize>()
        .min(usize::from(u16::MAX)) as u16
}

/// Backwards-compatible name for hard cell wrapping without word trimming.
pub fn wrapped_line_count(text: &str, width: u16) -> u16 {
    hard_wrapped_line_count(text, width)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_wrapping_newlines_and_wide_characters() {
        assert_eq!(wrapped_line_count("", 10), 1);
        assert_eq!(wrapped_line_count("123456", 3), 2);
        assert_eq!(wrapped_line_count("a\nb", 10), 2);
        assert_eq!(wrapped_line_count("界界", 2), 2);
    }

    #[test]
    fn zero_width_has_no_renderable_rows() {
        assert_eq!(wrapped_line_count("content", 0), 0);
    }
}
