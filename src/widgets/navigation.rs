#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DisabledFocusPolicy {
    Include,
    #[default]
    Skip,
}

pub(crate) fn adjacent_index(
    current: usize,
    length: usize,
    forward: bool,
    wrap: bool,
    mut can_focus: impl FnMut(usize) -> bool,
) -> usize {
    if length == 0 {
        return current;
    }

    let mut candidate = current;
    for _ in 0..length {
        candidate = if forward {
            if candidate + 1 == length {
                if wrap { 0 } else { return current }
            } else {
                candidate + 1
            }
        } else if candidate == 0 {
            if wrap { length - 1 } else { return current }
        } else {
            candidate - 1
        };

        if can_focus(candidate) {
            return candidate;
        }
    }

    current
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn navigation_wraps_and_can_skip_entries() {
        assert_eq!(adjacent_index(2, 3, true, true, |_| true), 0);
        assert_eq!(adjacent_index(0, 3, false, true, |_| true), 2);
        assert_eq!(adjacent_index(0, 3, true, true, |index| index == 2), 2);
    }

    #[test]
    fn navigation_stops_at_an_edge_when_wrapping_is_disabled() {
        assert_eq!(adjacent_index(0, 3, false, false, |_| true), 0);
        assert_eq!(adjacent_index(2, 3, true, false, |_| true), 2);
    }
}
