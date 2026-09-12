use std::{collections::HashSet, hash::Hash};

use super::{ChoiceVisualState, DisabledFocusPolicy, navigation::adjacent_index};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckboxItem<T> {
    pub value: T,
    pub label: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub disabled_reason: Option<String>,
}

/// Selection and focus state for a checkbox list.
///
/// `focus_next`, `focus_previous`, and `toggle_focused` provide the default
/// behavior normally mapped to arrow keys and Space. The group does not render
/// or register mouse regions; use `visual_state` with `render_choice_line`.
#[derive(Debug, Clone)]
pub struct CheckboxGroup<T: Clone + Eq + Hash> {
    items: Vec<CheckboxItem<T>>,
    selected: HashSet<T>,
    focused_index: Option<usize>,
    focus_policy: DisabledFocusPolicy,
    wrap_navigation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckboxGroupError {
    DuplicateValue,
    UnknownValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckboxChange<T> {
    Selected(T),
    Deselected(T),
    IgnoredDisabled(T),
    NoItem,
}

impl<T: Clone + Eq + Hash> CheckboxGroup<T> {
    pub fn new(
        items: Vec<CheckboxItem<T>>,
        selected: impl IntoIterator<Item = T>,
    ) -> Result<Self, CheckboxGroupError> {
        let mut values = HashSet::new();
        if items.iter().any(|item| !values.insert(item.value.clone())) {
            return Err(CheckboxGroupError::DuplicateValue);
        }
        let selected = selected
            .into_iter()
            .filter(|value| values.contains(value))
            .collect();
        let focused_index = items.iter().position(|item| item.enabled);
        Ok(Self {
            items,
            selected,
            focused_index,
            focus_policy: DisabledFocusPolicy::Skip,
            wrap_navigation: true,
        })
    }

    pub fn items(&self) -> &[CheckboxItem<T>] {
        &self.items
    }

    pub fn selected(&self) -> &HashSet<T> {
        &self.selected
    }

    pub fn focused_item(&self) -> Option<&CheckboxItem<T>> {
        self.focused_index.and_then(|index| self.items.get(index))
    }

    pub fn set_focus_policy(&mut self, policy: DisabledFocusPolicy) {
        self.focus_policy = policy;
        self.repair_focus();
    }

    pub fn set_wrap_navigation(&mut self, wrap: bool) {
        self.wrap_navigation = wrap;
    }

    pub fn focus_next(&mut self) {
        self.move_focus(true);
    }

    pub fn focus_previous(&mut self) {
        self.move_focus(false);
    }

    fn move_focus(&mut self, forward: bool) {
        let Some(current) = self.focused_index else {
            self.repair_focus();
            return;
        };
        self.focused_index = Some(adjacent_index(
            current,
            self.items.len(),
            forward,
            self.wrap_navigation,
            |index| self.focus_policy == DisabledFocusPolicy::Include || self.items[index].enabled,
        ));
    }

    pub fn toggle_focused(&mut self) -> CheckboxChange<T> {
        let Some(item) = self.focused_item() else {
            return CheckboxChange::NoItem;
        };
        let value = item.value.clone();
        if !item.enabled {
            return CheckboxChange::IgnoredDisabled(value);
        }
        if self.selected.remove(&value) {
            CheckboxChange::Deselected(value)
        } else {
            self.selected.insert(value.clone());
            CheckboxChange::Selected(value)
        }
    }

    pub fn set_enabled(&mut self, value: &T, enabled: bool) -> Result<(), CheckboxGroupError> {
        let item = self
            .items
            .iter_mut()
            .find(|item| &item.value == value)
            .ok_or(CheckboxGroupError::UnknownValue)?;
        item.enabled = enabled;
        self.repair_focus();
        Ok(())
    }

    pub fn set_selected(&mut self, value: T, selected: bool) -> Result<(), CheckboxGroupError> {
        if !self.items.iter().any(|item| item.value == value) {
            return Err(CheckboxGroupError::UnknownValue);
        }
        if selected {
            self.selected.insert(value);
        } else {
            self.selected.remove(&value);
        }
        Ok(())
    }

    pub fn visual_state(&self, value: &T) -> ChoiceVisualState {
        let item_index = self.items.iter().position(|item| &item.value == value);
        ChoiceVisualState {
            selected: self.selected.contains(value),
            focused: item_index == self.focused_index,
            enabled: item_index
                .and_then(|index| self.items.get(index))
                .is_some_and(|item| item.enabled),
        }
    }

    fn repair_focus(&mut self) {
        if self.items.is_empty() {
            self.focused_index = None;
            return;
        }
        if self.focus_policy == DisabledFocusPolicy::Include {
            self.focused_index.get_or_insert(0);
            return;
        }
        if self
            .focused_index
            .and_then(|index| self.items.get(index))
            .is_some_and(|item| item.enabled)
        {
            return;
        }
        let start = self.focused_index.unwrap_or(0).min(self.items.len());
        self.focused_index = self.items[start..]
            .iter()
            .position(|item| item.enabled)
            .map(|offset| start + offset)
            .or_else(|| self.items[..start].iter().rposition(|item| item.enabled));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(value: u8, enabled: bool) -> CheckboxItem<u8> {
        CheckboxItem {
            value,
            label: value.to_string(),
            description: None,
            enabled,
            disabled_reason: None,
        }
    }

    #[test]
    fn selection_and_disabled_focus_are_independent() {
        let mut group =
            CheckboxGroup::new(vec![item(1, true), item(2, false), item(3, true)], [1, 99])
                .unwrap();
        assert_eq!(group.selected(), &HashSet::from([1]));
        assert_eq!(group.toggle_focused(), CheckboxChange::Deselected(1));
        group.focus_next();
        assert_eq!(group.focused_item().map(|item| item.value), Some(3));
        group.set_focus_policy(DisabledFocusPolicy::Include);
        group.focus_previous();
        assert_eq!(group.toggle_focused(), CheckboxChange::IgnoredDisabled(2));
    }

    #[test]
    fn rejects_duplicate_values_and_ignores_unknown_selection() {
        assert!(matches!(
            CheckboxGroup::new(vec![item(1, true), item(1, true)], []),
            Err(CheckboxGroupError::DuplicateValue)
        ));
        let mut group = CheckboxGroup::new(vec![item(1, true)], []).unwrap();
        assert_eq!(
            group.set_selected(2, true),
            Err(CheckboxGroupError::UnknownValue)
        );
        assert!(group.selected().is_empty());
    }

    #[test]
    fn empty_group_reports_no_focused_item() {
        let mut group = CheckboxGroup::<u8>::new(Vec::new(), []).unwrap();
        assert_eq!(group.focused_item(), None);
        assert_eq!(group.toggle_focused(), CheckboxChange::NoItem);
    }

    #[test]
    fn disabling_last_focused_item_repairs_focus_without_wrapping() {
        let mut group = CheckboxGroup::new(vec![item(1, true), item(2, true)], []).unwrap();
        group.focus_next();
        group.set_wrap_navigation(false);
        group.set_enabled(&2, false).unwrap();
        assert_eq!(group.focused_item().map(|item| item.value), Some(1));

        group.set_enabled(&1, false).unwrap();
        assert_eq!(group.focused_item(), None);
    }
}
