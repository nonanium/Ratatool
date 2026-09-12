use ratatui::layout::{Position, Rect};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HitRegion<A> {
    pub area: Rect,
    pub action: A,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HitMap<A> {
    regions: Vec<HitRegion<A>>,
}

impl<A> HitMap<A> {
    pub fn new() -> Self {
        Self {
            regions: Vec::new(),
        }
    }

    pub fn register(&mut self, area: Rect, action: A) {
        if !area.is_empty() {
            self.regions.push(HitRegion { area, action });
        }
    }

    pub fn hit(&self, column: u16, row: u16) -> Option<&A> {
        let position = Position::new(column, row);
        self.regions
            .iter()
            .rev()
            .find(|region| region.area.contains(position))
            .map(|region| &region.action)
    }

    pub fn clear(&mut self) {
        self.regions.clear();
    }

    pub fn regions(&self) -> &[HitRegion<A>] {
        &self.regions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newest_overlapping_region_wins() {
        let mut map = HitMap::new();
        map.register(Rect::new(0, 0, 4, 4), "base");
        map.register(Rect::new(1, 1, 2, 2), "overlay");
        assert_eq!(map.hit(1, 1), Some(&"overlay"));
        assert_eq!(map.hit(3, 3), Some(&"base"));
        assert_eq!(map.hit(4, 4), None);
    }
}
