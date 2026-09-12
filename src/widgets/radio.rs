use super::{ChoiceVisualState, DisabledFocusPolicy, navigation::adjacent_index};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadioItem<T> {
    pub value: T,
    pub label: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub disabled_reason: Option<String>,
}

/// Selection and focus state for a non-empty radio list.
///
/// `focus_next`, `focus_previous`, and `select_focused` provide the default
/// behavior normally mapped to arrow keys and Space or Enter. The group does
/// not render or register mouse regions; use `visual_state` with
/// `render_choice_line`.
#[derive(Debug, Clone)]
pub struct RadioGroup<T: Clone + Eq> {
    items: Vec<RadioItem<T>>,
    selected: T,
    default: T,
    focused_index: usize,
    focus_policy: DisabledFocusPolicy,
    wrap_navigation: bool,
    disabled_selection_policy: DisabledSelectionPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RadioGroupError {
    Empty,
    DuplicateValue,
    DefaultMissing,
    DefaultDisabled,
    NoEnabledItems,
    SelectedItemDisabled,
    UnknownValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RadioChange<T> {
    Changed { previous: T, selected: T },
    Unchanged(T),
    IgnoredDisabled(T),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisabledSelectionPolicy {
    UseConfiguredDefault,
    UseFirstEnabled,
    ReturnError,
}

impl<T: Clone + Eq> RadioGroup<T> {
    pub fn new(
        items: Vec<RadioItem<T>>,
        observed: Option<T>,
        default: T,
    ) -> Result<Self, RadioGroupError> {
        if items.is_empty() {
            return Err(RadioGroupError::Empty);
        }
        if items
            .iter()
            .enumerate()
            .any(|(index, item)| items[..index].iter().any(|other| other.value == item.value))
        {
            return Err(RadioGroupError::DuplicateValue);
        }
        let default_item = items
            .iter()
            .find(|item| item.value == default)
            .ok_or(RadioGroupError::DefaultMissing)?;
        if !default_item.enabled {
            return Err(RadioGroupError::DefaultDisabled);
        }
        let focused_index = items
            .iter()
            .position(|item| item.enabled)
            .ok_or(RadioGroupError::NoEnabledItems)?;
        let selected = observed
            .filter(|value| {
                items
                    .iter()
                    .any(|item| item.enabled && item.value == *value)
            })
            .unwrap_or_else(|| default.clone());
        Ok(Self {
            items,
            selected,
            default,
            focused_index,
            focus_policy: DisabledFocusPolicy::Skip,
            wrap_navigation: true,
            disabled_selection_policy: DisabledSelectionPolicy::UseConfiguredDefault,
        })
    }

    pub fn items(&self) -> &[RadioItem<T>] {
        &self.items
    }

    pub fn selected(&self) -> &T {
        &self.selected
    }

    pub fn default(&self) -> &T {
        &self.default
    }

    pub fn focused_item(&self) -> &RadioItem<T> {
        // Construction guarantees a non-empty item list and every update keeps it intact.
        &self.items[self.focused_index]
    }

    pub fn focus_next(&mut self) {
        self.move_focus(true);
    }

    pub fn focus_previous(&mut self) {
        self.move_focus(false);
    }

    fn move_focus(&mut self, forward: bool) {
        self.focused_index = adjacent_index(
            self.focused_index,
            self.items.len(),
            forward,
            self.wrap_navigation,
            |index| self.focus_policy == DisabledFocusPolicy::Include || self.items[index].enabled,
        );
    }

    pub fn select_focused(&mut self) -> RadioChange<T> {
        let item = self.focused_item();
        let value = item.value.clone();
        if !item.enabled {
            return RadioChange::IgnoredDisabled(value);
        }
        if value == self.selected {
            RadioChange::Unchanged(value)
        } else {
            let previous = std::mem::replace(&mut self.selected, value);
            RadioChange::Changed {
                previous,
                selected: self.selected.clone(),
            }
        }
    }

    pub fn visual_state(&self, value: &T) -> ChoiceVisualState {
        let item_index = self.items.iter().position(|item| &item.value == value);
        ChoiceVisualState {
            selected: &self.selected == value,
            focused: item_index == Some(self.focused_index),
            enabled: item_index
                .and_then(|index| self.items.get(index))
                .is_some_and(|item| item.enabled),
        }
    }

    pub fn set_disabled_selection_policy(&mut self, policy: DisabledSelectionPolicy) {
        self.disabled_selection_policy = policy;
    }

    pub fn set_focus_policy(&mut self, policy: DisabledFocusPolicy) {
        self.focus_policy = policy;
        self.repair_focus();
    }

    pub fn set_wrap_navigation(&mut self, wrap: bool) {
        self.wrap_navigation = wrap;
    }

    pub fn set_enabled(&mut self, value: &T, enabled: bool) -> Result<(), RadioGroupError> {
        let Some(index) = self.items.iter().position(|item| &item.value == value) else {
            return Err(RadioGroupError::UnknownValue);
        };
        if self.items[index].enabled == enabled {
            return Ok(());
        }
        if !enabled
            && self
                .items
                .iter()
                .enumerate()
                .all(|(other, item)| other == index || !item.enabled)
        {
            return Err(RadioGroupError::NoEnabledItems);
        }
        if !enabled && self.selected == *value {
            let replacement = match self.disabled_selection_policy {
                DisabledSelectionPolicy::UseConfiguredDefault if self.default != *value => self
                    .items
                    .iter()
                    .find(|item| item.enabled && item.value == self.default)
                    .map(|item| item.value.clone()),
                DisabledSelectionPolicy::UseConfiguredDefault => None,
                DisabledSelectionPolicy::UseFirstEnabled => self
                    .items
                    .iter()
                    .enumerate()
                    .find(|(other, item)| *other != index && item.enabled)
                    .map(|(_, item)| item.value.clone()),
                DisabledSelectionPolicy::ReturnError => {
                    return Err(RadioGroupError::SelectedItemDisabled);
                }
            };
            self.selected = replacement.ok_or(RadioGroupError::SelectedItemDisabled)?;
        }
        self.items[index].enabled = enabled;
        self.repair_focus();
        Ok(())
    }

    fn repair_focus(&mut self) {
        if self.focus_policy == DisabledFocusPolicy::Include
            || self.items[self.focused_index].enabled
        {
            return;
        }
        if let Some(index) = self.items.iter().position(|item| item.enabled) {
            self.focused_index = index;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(value: u8, enabled: bool) -> RadioItem<u8> {
        RadioItem {
            value,
            label: value.to_string(),
            description: None,
            enabled,
            disabled_reason: None,
        }
    }

    #[test]
    fn validates_items_and_preserves_one_selection() {
        assert!(matches!(
            RadioGroup::new(Vec::<RadioItem<u8>>::new(), None, 1),
            Err(RadioGroupError::Empty)
        ));
        assert!(matches!(
            RadioGroup::new(vec![item(1, true), item(1, true)], None, 1),
            Err(RadioGroupError::DuplicateValue)
        ));
        assert!(matches!(
            RadioGroup::new(vec![item(1, true)], None, 2),
            Err(RadioGroupError::DefaultMissing)
        ));
        assert!(matches!(
            RadioGroup::new(vec![item(1, false)], None, 1),
            Err(RadioGroupError::DefaultDisabled)
        ));

        let mut group = RadioGroup::new(vec![item(1, true), item(2, true)], Some(2), 1).unwrap();
        group.focus_next();
        assert_eq!(group.select_focused(), RadioChange::Unchanged(2));
        group.focus_previous();
        assert_eq!(
            group.select_focused(),
            RadioChange::Changed {
                previous: 2,
                selected: 1
            }
        );
    }

    #[test]
    fn disabling_selected_item_obeys_policy_without_partial_mutation() {
        let mut group = RadioGroup::new(vec![item(1, true), item(2, true)], Some(2), 1).unwrap();
        group.set_enabled(&2, false).unwrap();
        assert_eq!(group.selected(), &1);

        group.set_enabled(&2, true).unwrap();
        group.focus_next();
        group.select_focused();
        group.set_disabled_selection_policy(DisabledSelectionPolicy::ReturnError);
        assert_eq!(
            group.set_enabled(&2, false),
            Err(RadioGroupError::SelectedItemDisabled)
        );
        assert_eq!(group.selected(), &2);
        assert!(group.items()[1].enabled);
    }

    #[test]
    fn include_policy_can_focus_but_not_select_disabled_item() {
        let mut group = RadioGroup::new(vec![item(1, true), item(2, false)], None, 1).unwrap();
        group.set_focus_policy(DisabledFocusPolicy::Include);
        group.focus_next();
        assert_eq!(group.select_focused(), RadioChange::IgnoredDisabled(2));
    }

    #[test]
    fn disabling_last_focused_item_repairs_focus_without_wrapping() {
        let mut group = RadioGroup::new(vec![item(1, true), item(2, true)], None, 1).unwrap();
        group.focus_next();
        group.set_wrap_navigation(false);
        group.set_enabled(&2, false).unwrap();
        assert_eq!(group.focused_item().value, 1);
        assert_eq!(
            group.set_enabled(&99, false),
            Err(RadioGroupError::UnknownValue)
        );
    }
}
