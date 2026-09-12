use ratatui::layout::Rect;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequiredSize {
    pub width: u16,
    pub height: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FitLevel {
    Preferred,
    Compact,
    Fallback,
}

pub fn select_fit_level(area: Rect, preferred: RequiredSize, compact: RequiredSize) -> FitLevel {
    if area.width >= preferred.width && area.height >= preferred.height {
        FitLevel::Preferred
    } else if area.width >= compact.width && area.height >= compact.height {
        FitLevel::Compact
    } else {
        FitLevel::Fallback
    }
}

pub fn centered_rect(area: Rect, maximum: RequiredSize) -> Rect {
    let width = area.width.min(maximum.width);
    let height = area.height.min(maximum.height);
    Rect {
        x: area.x.saturating_add(area.width.saturating_sub(width) / 2),
        y: area
            .y
            .saturating_add(area.height.saturating_sub(height) / 2),
        width,
        height,
    }
}

pub fn reserve_vertical(area: Rect, top: u16, bottom: u16) -> Option<Rect> {
    let height = area.height.checked_sub(top)?.checked_sub(bottom)?;
    Some(Rect {
        x: area.x,
        y: area.y.checked_add(top)?,
        width: area.width,
        height,
    })
}

pub fn inset_checked(area: Rect, horizontal: u16, vertical: u16) -> Option<Rect> {
    let width = area.width.checked_sub(horizontal.checked_mul(2)?)?;
    let height = area.height.checked_sub(vertical.checked_mul(2)?)?;
    Some(Rect {
        x: area.x.checked_add(horizontal)?,
        y: area.y.checked_add(vertical)?,
        width,
        height,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chooses_fit_from_both_dimensions() {
        let preferred = RequiredSize {
            width: 100,
            height: 40,
        };
        let compact = RequiredSize {
            width: 60,
            height: 20,
        };
        assert_eq!(
            select_fit_level(Rect::new(0, 0, 100, 40), preferred, compact),
            FitLevel::Preferred
        );
        assert_eq!(
            select_fit_level(Rect::new(0, 0, 80, 20), preferred, compact),
            FitLevel::Compact
        );
        assert_eq!(
            select_fit_level(Rect::new(0, 0, 100, 10), preferred, compact),
            FitLevel::Fallback
        );
    }

    #[test]
    fn centers_and_clamps_rect() {
        assert_eq!(
            centered_rect(
                Rect::new(10, 20, 80, 30),
                RequiredSize {
                    width: 40,
                    height: 10,
                }
            ),
            Rect::new(30, 30, 40, 10)
        );
        assert_eq!(
            centered_rect(
                Rect::new(10, 20, 10, 5),
                RequiredSize {
                    width: 40,
                    height: 10,
                }
            ),
            Rect::new(10, 20, 10, 5)
        );
    }

    #[test]
    fn checked_layout_returns_none_when_space_is_insufficient() {
        let area = Rect::new(0, 0, 10, 5);
        assert_eq!(reserve_vertical(area, 2, 3), Some(Rect::new(0, 2, 10, 0)));
        assert_eq!(reserve_vertical(area, 3, 3), None);
        assert_eq!(inset_checked(area, 5, 0), Some(Rect::new(5, 0, 0, 5)));
        assert_eq!(inset_checked(area, 6, 0), None);
    }
}
