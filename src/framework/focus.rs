use std::{collections::HashMap, hash::Hash};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FocusEntry<K> {
    pub key: K,
    pub enabled: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FocusError {
    UnknownKey,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FocusManager<K> {
    entries: Vec<FocusEntry<K>>,
    current: Option<usize>,
    wrap: bool,
}

impl<K: Eq> FocusManager<K> {
    pub fn new(entries: Vec<FocusEntry<K>>) -> Self {
        let current = entries.iter().position(|entry| entry.enabled);
        Self {
            entries,
            current,
            wrap: true,
        }
    }

    pub fn current(&self) -> Option<&K> {
        self.current.map(|index| &self.entries[index].key)
    }

    pub fn set_wrap(&mut self, wrap: bool) {
        self.wrap = wrap;
    }

    pub fn focus(&mut self, key: &K) -> Result<bool, FocusError> {
        let index = self
            .entries
            .iter()
            .position(|entry| &entry.key == key && entry.enabled)
            .ok_or(FocusError::UnknownKey)?;
        Ok(self.replace_current(Some(index)))
    }

    pub fn set_enabled(&mut self, key: &K, enabled: bool) -> Result<(), FocusError> {
        let index = self
            .entries
            .iter()
            .position(|entry| &entry.key == key)
            .ok_or(FocusError::UnknownKey)?;
        self.entries[index].enabled = enabled;
        if self.current == Some(index) && !enabled {
            self.current = self.entries.iter().position(|entry| entry.enabled);
        } else if self.current.is_none() && enabled {
            self.current = Some(index);
        }
        Ok(())
    }

    pub fn focus_next(&mut self) -> bool {
        self.move_by(true)
    }

    pub fn focus_previous(&mut self) -> bool {
        self.move_by(false)
    }

    fn move_by(&mut self, forward: bool) -> bool {
        let Some(current) = self.current else {
            let target = self.entries.iter().position(|entry| entry.enabled);
            return self.replace_current(target);
        };
        let length = self.entries.len();
        let mut candidate = current;
        for _ in 0..length {
            candidate = if forward {
                if candidate + 1 == length {
                    if !self.wrap {
                        break;
                    }
                    0
                } else {
                    candidate + 1
                }
            } else if candidate == 0 {
                if !self.wrap {
                    break;
                }
                length - 1
            } else {
                candidate - 1
            };
            if self.entries[candidate].enabled {
                return self.replace_current(Some(candidate));
            }
        }
        false
    }

    fn replace_current(&mut self, current: Option<usize>) -> bool {
        if self.current == current {
            false
        } else {
            self.current = current;
            true
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Clone, Debug, Default)]
pub struct FocusGraph<K> {
    edges: HashMap<(K, Direction), K>,
}

impl<K: Clone + Eq + Hash> FocusGraph<K> {
    pub fn connect(&mut self, from: K, direction: Direction, to: K) -> Option<K> {
        self.edges.insert((from, direction), to)
    }

    pub fn target(&self, from: &K, direction: Direction) -> Option<&K> {
        self.edges.get(&(from.clone(), direction))
    }

    pub fn connect_bidirectional(&mut self, first: K, direction: Direction, second: K) {
        let reverse = match direction {
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
        };
        self.connect(first.clone(), direction, second.clone());
        self.connect(second, reverse, first);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manager_skips_disabled_and_repairs_focus() {
        let mut focus = FocusManager::new(vec![
            FocusEntry {
                key: 1,
                enabled: true,
            },
            FocusEntry {
                key: 2,
                enabled: false,
            },
            FocusEntry {
                key: 3,
                enabled: true,
            },
        ]);
        assert!(focus.focus_next());
        assert_eq!(focus.current(), Some(&3));
        focus.set_enabled(&3, false).unwrap();
        assert_eq!(focus.current(), Some(&1));
    }

    #[test]
    fn graph_supports_directional_links() {
        let mut graph = FocusGraph::default();
        graph.connect_bidirectional("left", Direction::Right, "right");
        assert_eq!(graph.target(&"left", Direction::Right), Some(&"right"));
        assert_eq!(graph.target(&"right", Direction::Left), Some(&"left"));
    }
}
