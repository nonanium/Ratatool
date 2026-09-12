use std::{
    io,
    sync::{Arc, Mutex, MutexGuard},
    time::{Duration, Instant},
};

use crossterm::event::{KeyCode, KeyEvent, MouseEventKind};
use ratatool::{
    AsyncEventSource, Button, ButtonKind, ButtonTheme, CheckboxGroup, CheckboxItem, ChoiceKind,
    FocusEntry, FocusManager, Interaction, MenuItem, MenuRenderOptions, MenuResult, MenuState,
    Notification, NotificationLevel, NotificationOverlay, Panel, PanelPresentation, RadioGroup,
    RadioItem, Runtime, Screen, ScreenPresentation, ScrollOptions, ScrollResult, ScrollState,
    SelectionTheme, SessionOptions, SurfaceTheme, TerminalProfile, TerminalSession,
    TextInputConfig, TextInputResult, TextInputState, TextTheme, ThemeFamily, ThemeId,
    ThemeMetrics, ThemeRegistry, ThemeState, ThemeVariant, UiEvent, UiFrame, render_choice_line,
    render_scroll,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
};

#[derive(Clone, Debug, PartialEq, Eq)]
enum Action {
    OpenDetails,
    Back,
    OpenDialog,
    CloseDialog,
    AddNotification,
    SwitchTheme,
    SwitchProfile,
    Logs,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Control {
    Menu,
    Radio,
    Checkbox,
    Input,
}

struct ShowcaseState {
    focus: FocusManager<Control>,
    menu: MenuState<Action>,
    radio: RadioGroup<u8>,
    checkboxes: CheckboxGroup<u8>,
    input: TextInputState,
    logs: ScrollState,
    notifications: NotificationOverlay,
    theme_index: usize,
    profile_index: usize,
    next_notification_id: u64,
}

type SharedState = Arc<Mutex<ShowcaseState>>;

fn lock_state(state: &SharedState) -> MutexGuard<'_, ShowcaseState> {
    state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl ShowcaseState {
    fn new() -> io::Result<Self> {
        let radio = RadioGroup::new(
            vec![radio_item(1, "Local"), radio_item(2, "Remote")],
            None,
            1,
        )
        .map_err(|error| io::Error::other(format!("invalid showcase radio group: {error:?}")))?;
        let checkboxes = CheckboxGroup::new(
            vec![checkbox_item(1, "Mouse"), checkbox_item(2, "Notifications")],
            [1, 2],
        )
        .map_err(|error| io::Error::other(format!("invalid showcase checkbox group: {error:?}")))?;
        Ok(Self {
            focus: FocusManager::new(vec![
                FocusEntry {
                    key: Control::Menu,
                    enabled: true,
                },
                FocusEntry {
                    key: Control::Radio,
                    enabled: true,
                },
                FocusEntry {
                    key: Control::Checkbox,
                    enabled: true,
                },
                FocusEntry {
                    key: Control::Input,
                    enabled: true,
                },
            ]),
            menu: MenuState::new(vec![
                MenuItem {
                    label: "Dashboard".into(),
                    description: Some("current screen".into()),
                    value: Action::OpenDetails,
                    enabled: true,
                    disabled_reason: None,
                },
                MenuItem {
                    label: "Details".into(),
                    description: Some("push replacement".into()),
                    value: Action::OpenDetails,
                    enabled: true,
                    disabled_reason: None,
                },
                MenuItem {
                    label: "Unavailable".into(),
                    description: None,
                    value: Action::OpenDetails,
                    enabled: false,
                    disabled_reason: Some("showcase disabled state".into()),
                },
            ]),
            radio,
            checkboxes,
            input: TextInputState::new("Graphemes: e\u{301} 界"),
            logs: ScrollState::default(),
            notifications: NotificationOverlay::default(),
            theme_index: 0,
            profile_index: 0,
            next_notification_id: 1,
        })
    }

    fn is_focused(&self, control: Control) -> bool {
        self.focus.current() == Some(&control)
    }
}

struct Dashboard {
    state: SharedState,
}

impl Screen<Action> for Dashboard {
    fn render(&mut self, ui: &mut UiFrame<'_, '_, Action>, area: Rect) {
        render_shell(ui, area, "Dashboard", &self.state, |ui, content, state| {
            let columns = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(44), Constraint::Percentage(56)])
                .split(content);
            let left = ui.panel(columns[0], &Panel::new().title(" Controls ").padding(1));
            let left_rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(4),
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Length(2),
                    Constraint::Min(0),
                ])
                .split(left);

            ui.menu(
                left_rows[0],
                &state.menu,
                MenuRenderOptions {
                    focused: state.is_focused(Control::Menu),
                },
            );
            render_radio(ui, left_rows[1], state);
            render_checkboxes(ui, left_rows[2], state);
            let muted = ui.theme().text.muted;
            ui.frame().render_widget(
                Paragraph::new("Input").style(muted),
                Rect::new(left_rows[3].x, left_rows[3].y, left_rows[3].width, 1),
            );
            ui.text_input(
                Rect::new(
                    left_rows[3].x,
                    left_rows[3].y.saturating_add(1),
                    left_rows[3].width,
                    1,
                ),
                &state.input,
                &TextInputConfig::new()
                    .placeholder("Type here")
                    .focused(state.is_focused(Control::Input)),
            );

            let log_area = ui.panel(columns[1], &Panel::new().title(" Logs ").padding(1));
            let lines: Vec<_> = (1..=24)
                .map(|line| Line::from(format!("{line:02}  background operation completed")))
                .collect();
            {
                let (frame, theme) = ui.frame_and_theme();
                render_scroll(
                    frame,
                    log_area,
                    &state.logs,
                    ScrollOptions {
                        show_scrollbar: true,
                        focused: false,
                    },
                    Paragraph::new(lines),
                    24,
                    theme,
                );
            }
            ui.hit_map().register(log_area, Action::Logs);
        });
    }

    fn handle_event(&mut self, event: &UiEvent<Action>) -> Interaction<Action> {
        let mut state = lock_state(&self.state);
        match event {
            UiEvent::Key(key)
                if key.code == KeyCode::Esc
                    || (key.code == KeyCode::Char('q') && !state.is_focused(Control::Input)) =>
            {
                Interaction::Exit
            }
            UiEvent::Key(key) if key.code == KeyCode::Tab => Interaction::Handled {
                changed: state.focus.focus_next(),
                message: None,
            },
            UiEvent::Key(key) if key.code == KeyCode::BackTab => Interaction::Handled {
                changed: state.focus.focus_previous(),
                message: None,
            },
            UiEvent::Key(key) if key.code == KeyCode::F(4) => {
                Interaction::message(Action::OpenDialog)
            }
            UiEvent::Key(key) if key.code == KeyCode::F(5) => {
                Interaction::message(Action::AddNotification)
            }
            UiEvent::Key(key) if key.code == KeyCode::F(2) => {
                Interaction::message(Action::SwitchTheme)
            }
            UiEvent::Key(key) if key.code == KeyCode::F(3) => {
                Interaction::message(Action::SwitchProfile)
            }
            UiEvent::Key(key) => handle_control_key(&mut state, *key),
            UiEvent::Paste(text) if state.is_focused(Control::Input) => {
                match state.input.insert_text(text) {
                    TextInputResult::Changed => Interaction::changed(),
                    TextInputResult::Unchanged => Interaction::unchanged(),
                    TextInputResult::Moved | TextInputResult::Unhandled => Interaction::Unhandled,
                }
            }
            UiEvent::Pointer { event, hit, .. }
                if matches!(hit, Some(Action::Logs))
                    && matches!(event.kind, MouseEventKind::ScrollUp) =>
            {
                scroll_interaction(state.logs.up(1))
            }
            UiEvent::Pointer { event, hit, .. }
                if matches!(hit, Some(Action::Logs))
                    && matches!(event.kind, MouseEventKind::ScrollDown) =>
            {
                scroll_interaction(state.logs.down(1, 24, 8))
            }
            UiEvent::Tick => Interaction::Handled {
                changed: state.notifications.expire(Instant::now()),
                message: None,
            },
            _ => Interaction::Unhandled,
        }
    }
}

struct Details {
    state: SharedState,
}

impl Screen<Action> for Details {
    fn render(&mut self, ui: &mut UiFrame<'_, '_, Action>, area: Rect) {
        render_shell(ui, area, "Details", &self.state, |ui, content, _| {
            let inner = ui.panel(
                content,
                &Panel::new().title(" Replacement screen ").padding(1),
            );
            ui.frame().render_widget(
                Paragraph::new(vec![
                    Line::from("Only this screen is rendered by the replacement layer."),
                    Line::from(""),
                    Line::from("Press Esc to pop it or F4 to open a modal overlay."),
                ]),
                inner,
            );
        });
    }

    fn handle_event(&mut self, event: &UiEvent<Action>) -> Interaction<Action> {
        match event {
            UiEvent::Key(key) if matches!(key.code, KeyCode::Esc | KeyCode::Backspace) => {
                Interaction::message(Action::Back)
            }
            UiEvent::Key(key) if key.code == KeyCode::F(4) => {
                Interaction::message(Action::OpenDialog)
            }
            UiEvent::Tick => Interaction::Handled {
                changed: lock_state(&self.state).notifications.expire(Instant::now()),
                message: None,
            },
            _ => Interaction::Unhandled,
        }
    }
}

struct ConfirmDialog {
    dialog: ratatool::Dialog<'static, Action>,
}

impl ConfirmDialog {
    fn new() -> Self {
        let mut dialog = ratatool::Dialog::new(
            "Modal overlay",
            "Underlying pixels remain visible, while underlying hit regions are blocked.",
        );
        dialog.push_button(
            Button::new("Close").kind(ButtonKind::Cancel),
            Action::CloseDialog,
        );
        dialog.cancel_action = Some(Action::CloseDialog);
        Self { dialog }
    }
}

impl Screen<Action> for ConfirmDialog {
    fn render(&mut self, ui: &mut UiFrame<'_, '_, Action>, area: Rect) {
        ui.dialog(area, &self.dialog);
    }

    fn handle_event(&mut self, event: &UiEvent<Action>) -> Interaction<Action> {
        match event {
            UiEvent::Key(key) => self.dialog.handle_key(*key),
            _ => Interaction::Unhandled,
        }
    }
}

fn render_shell(
    ui: &mut UiFrame<'_, '_, Action>,
    area: Rect,
    title: &str,
    state: &SharedState,
    render_content: impl FnOnce(&mut UiFrame<'_, '_, Action>, Rect, &ShowcaseState),
) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(3),
            Constraint::Length(1),
        ])
        .split(area);
    let state = lock_state(state);
    let background = ui.theme().surfaces.background;
    ui.frame()
        .render_widget(Block::new().style(background), area);
    let heading = ui.theme().text.heading;
    let muted = ui.theme().text.muted;
    let toolbar = ui.theme().surfaces.toolbar;
    let header = Paragraph::new(Line::from(vec![
        Span::styled(format!(" Ratatool / {title}"), heading),
        Span::styled(
            format!(
                "  {:?} / {:?}",
                profile_at(state.profile_index).render_class(),
                profile_at(state.profile_index).charset
            ),
            muted,
        ),
    ]))
    .style(toolbar);
    ui.frame().render_widget(header, rows[0]);
    let button_width = 14.min(rows[0].width / 3);
    let right = rows[0].right();
    ui.button(
        Rect::new(
            right.saturating_sub(button_width * 3),
            rows[0].y,
            button_width,
            1,
        ),
        &Button::new("Theme").kind(ButtonKind::Secondary),
        Some(Action::SwitchTheme),
    );
    ui.button(
        Rect::new(
            right.saturating_sub(button_width * 2),
            rows[0].y,
            button_width,
            1,
        ),
        &Button::new("Profile").kind(ButtonKind::Secondary),
        Some(Action::SwitchProfile),
    );
    ui.button(
        Rect::new(
            right.saturating_sub(button_width),
            rows[0].y,
            button_width,
            1,
        ),
        &Button::new("Dialog"),
        Some(Action::OpenDialog),
    );

    render_content(ui, rows[1], &state);
    let footer_style = ui.theme().surfaces.footer.patch(ui.theme().text.muted);
    let footer =
        Paragraph::new(" Tab focus  F2 theme  F3 profile  F4 dialog  F5 notify  Esc quit ")
            .style(footer_style);
    ui.frame().render_widget(footer, rows[2]);
    ui.notifications(rows[1], &state.notifications);
}

fn render_radio(ui: &mut UiFrame<'_, '_, Action>, area: Rect, state: &ShowcaseState) {
    for (index, item) in state.radio.items().iter().enumerate() {
        let mut visual = state.radio.visual_state(&item.value);
        visual.focused &= state.is_focused(Control::Radio);
        let line = render_choice_line(&item.label, ChoiceKind::Radio, visual, ui.theme());
        ui.frame().render_widget(
            Paragraph::new(line),
            Rect::new(area.x, area.y.saturating_add(index as u16), area.width, 1),
        );
    }
}

fn render_checkboxes(ui: &mut UiFrame<'_, '_, Action>, area: Rect, state: &ShowcaseState) {
    for (index, item) in state.checkboxes.items().iter().enumerate() {
        let mut visual = state.checkboxes.visual_state(&item.value);
        visual.focused &= state.is_focused(Control::Checkbox);
        let line = render_choice_line(&item.label, ChoiceKind::Checkbox, visual, ui.theme());
        ui.frame().render_widget(
            Paragraph::new(line),
            Rect::new(area.x, area.y.saturating_add(index as u16), area.width, 1),
        );
    }
}

fn handle_control_key(state: &mut ShowcaseState, key: KeyEvent) -> Interaction<Action> {
    match state.focus.current() {
        Some(Control::Menu) => match state.menu.handle_key_in_viewport(key.code, 4) {
            MenuResult::Moved => Interaction::changed(),
            MenuResult::Activated(action) => Interaction::message(action),
            MenuResult::Unchanged | MenuResult::NoSelection => Interaction::unchanged(),
            MenuResult::Unhandled => Interaction::Unhandled,
        },
        Some(Control::Radio) => match key.code {
            KeyCode::Up => {
                state.radio.focus_previous();
                Interaction::changed()
            }
            KeyCode::Down => {
                state.radio.focus_next();
                Interaction::changed()
            }
            KeyCode::Char(' ') | KeyCode::Enter => {
                state.radio.select_focused();
                Interaction::changed()
            }
            _ => Interaction::Unhandled,
        },
        Some(Control::Checkbox) => match key.code {
            KeyCode::Up => {
                state.checkboxes.focus_previous();
                Interaction::changed()
            }
            KeyCode::Down => {
                state.checkboxes.focus_next();
                Interaction::changed()
            }
            KeyCode::Char(' ') | KeyCode::Enter => {
                state.checkboxes.toggle_focused();
                Interaction::changed()
            }
            _ => Interaction::Unhandled,
        },
        Some(Control::Input) => match state.input.handle_key(key) {
            TextInputResult::Changed | TextInputResult::Moved => Interaction::changed(),
            TextInputResult::Unchanged => Interaction::unchanged(),
            TextInputResult::Unhandled => Interaction::Unhandled,
        },
        None => Interaction::Unhandled,
    }
}

fn scroll_interaction(result: ScrollResult) -> Interaction<Action> {
    match result {
        ScrollResult::Changed => Interaction::changed(),
        ScrollResult::Unchanged => Interaction::unchanged(),
        ScrollResult::Unhandled => Interaction::Unhandled,
    }
}

fn radio_item(value: u8, label: &str) -> RadioItem<u8> {
    RadioItem {
        value,
        label: label.into(),
        description: None,
        enabled: true,
        disabled_reason: None,
    }
}

fn checkbox_item(value: u8, label: &str) -> CheckboxItem<u8> {
    CheckboxItem {
        value,
        label: label.into(),
        description: None,
        enabled: true,
        disabled_reason: None,
    }
}

fn profile_at(index: usize) -> TerminalProfile {
    const PROFILES: [TerminalProfile; 4] = [
        TerminalProfile::truecolor_unicode(),
        TerminalProfile::ansi16_unicode(),
        TerminalProfile::styled_ascii(),
        TerminalProfile::plain_ascii(),
    ];
    PROFILES[index % PROFILES.len()]
}

fn theme_families() -> Vec<ThemeFamily> {
    vec![
        theme_family("bordered", PanelPresentation::Bordered),
        theme_family("surface", PanelPresentation::Surface),
    ]
}

fn theme_family(id: &str, panel_presentation: PanelPresentation) -> ThemeFamily {
    let full_color = colorful_variant(panel_presentation, Color::Rgb(24, 28, 38));
    let color = colorful_variant(panel_presentation, Color::Indexed(236));
    let styled = monochrome_variant(panel_presentation, true);
    let plain = monochrome_variant(panel_presentation, false);
    ThemeFamily::new(ThemeId::new(id), full_color, color, styled, plain)
}

fn colorful_variant(panel_presentation: PanelPresentation, background: Color) -> ThemeVariant {
    ThemeVariant {
        text: TextTheme {
            normal: Style::default().fg(Color::White),
            muted: Style::default().fg(Color::Gray),
            emphasis: Style::default().fg(Color::Cyan),
            disabled: Style::default().fg(Color::DarkGray),
            heading: Style::default().fg(Color::Cyan).bold(),
            link: Style::default().fg(Color::LightBlue).underlined(),
            code: Style::default().fg(Color::LightYellow),
        },
        surfaces: SurfaceTheme {
            background: Style::default().bg(background),
            panel: Style::default().bg(Color::Rgb(38, 43, 56)),
            panel_focused: Style::default().bg(Color::Rgb(55, 61, 78)),
            panel_header: Style::default().bg(Color::Rgb(45, 52, 70)),
            raised: Style::default().bg(Color::Rgb(55, 61, 78)),
            toolbar: Style::default().bg(Color::Rgb(45, 52, 70)),
            footer: Style::default().bg(Color::Rgb(31, 36, 48)),
            overlay: Style::default().bg(Color::Rgb(55, 61, 78)),
        },
        buttons: ButtonTheme {
            primary: Style::default().fg(Color::Black).bg(Color::Cyan),
            primary_focused: Style::default()
                .fg(Color::Black)
                .bg(Color::LightCyan)
                .bold(),
            secondary: Style::default().fg(Color::White).bg(Color::DarkGray),
            secondary_focused: Style::default().fg(Color::Black).bg(Color::White),
            cancel: Style::default().fg(Color::White).bg(Color::Red),
            cancel_focused: Style::default().fg(Color::White).bg(Color::LightRed).bold(),
            destructive: Style::default().fg(Color::White).bg(Color::Red),
            destructive_focused: Style::default().fg(Color::White).bg(Color::LightRed).bold(),
            disabled: Style::default().fg(Color::DarkGray),
        },
        selection: SelectionTheme {
            normal: Style::default().fg(Color::White),
            selected: Style::default().fg(Color::LightGreen),
            focused: Style::default().add_modifier(Modifier::REVERSED),
            disabled: Style::default().fg(Color::DarkGray).dim(),
        },
        metrics: ThemeMetrics { panel_presentation },
        ..ThemeVariant::default()
    }
}

fn monochrome_variant(panel_presentation: PanelPresentation, styled: bool) -> ThemeVariant {
    let emphasis = if styled {
        Style::default().bold()
    } else {
        Style::default()
    };
    ThemeVariant {
        text: TextTheme {
            heading: emphasis,
            emphasis,
            disabled: if styled {
                Style::default().dim()
            } else {
                Style::default()
            },
            ..TextTheme::default()
        },
        selection: SelectionTheme {
            focused: if styled {
                Style::default().reversed()
            } else {
                Style::default()
            },
            ..SelectionTheme::default()
        },
        metrics: ThemeMetrics { panel_presentation },
        ..ThemeVariant::default()
    }
}

fn handle_action(
    action: Action,
    runtime: &mut Runtime<Action>,
    state: &SharedState,
    families: &[ThemeFamily],
    registry: &ThemeRegistry,
    theme: &mut ThemeState,
) -> io::Result<()> {
    match action {
        Action::OpenDetails => runtime.screens_mut().push(Details {
            state: Arc::clone(state),
        }),
        Action::Back | Action::CloseDialog => {
            runtime.screens_mut().pop();
        }
        Action::OpenDialog => runtime
            .screens_mut()
            .push_with_presentation(ConfirmDialog::new(), ScreenPresentation::ModalOverlay),
        Action::AddNotification => {
            let deadline = Instant::now() + Duration::from_secs(4);
            let mut state = lock_state(state);
            let id = state.next_notification_id;
            state.next_notification_id = state.next_notification_id.saturating_add(1);
            state.notifications.push(Notification {
                id,
                level: NotificationLevel::Success,
                message: "Background operation completed".into(),
                expires_at: Some(deadline),
            });
            runtime.request_redraw_at(deadline);
        }
        Action::SwitchTheme => {
            let mut state = lock_state(state);
            state.theme_index = (state.theme_index + 1) % families.len();
            theme
                .set_theme(families[state.theme_index].id.clone(), registry)
                .map_err(|error| io::Error::other(format!("failed to switch theme: {error}")))?;
        }
        Action::SwitchProfile => {
            let mut state = lock_state(state);
            state.profile_index = (state.profile_index + 1) % 4;
            theme
                .set_profile(profile_at(state.profile_index), registry)
                .map_err(|error| io::Error::other(format!("failed to switch profile: {error}")))?;
        }
        Action::Logs => {}
    }
    runtime.request_redraw();
    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> io::Result<()> {
    let state = Arc::new(Mutex::new(ShowcaseState::new()?));
    let families = theme_families();
    let mut registry = ThemeRegistry::new();
    for family in &families {
        registry.register(family.clone());
    }
    let mut theme = ThemeState::new(families[0].id.clone(), profile_at(0), &registry)
        .map_err(|error| io::Error::other(format!("invalid showcase theme: {error}")))?;
    let mut session = TerminalSession::enter(SessionOptions::default())?;
    let mut runtime = Runtime::new(Dashboard {
        state: Arc::clone(&state),
    });
    let (_messages, mut source) = AsyncEventSource::channel(32);

    while runtime.is_running() {
        runtime.draw_if_due(session.terminal_mut(), &theme)?;
        let Some(event) = source.next(runtime.next_redraw()).await? else {
            break;
        };
        if let Some(action) = runtime.dispatch(event) {
            handle_action(
                action,
                &mut runtime,
                &state,
                &families,
                &registry,
                &mut theme,
            )?;
        }
    }
    Ok(())
}
