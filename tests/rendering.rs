use std::{
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatool::{
    Button, Interaction, Runtime, Screen, ScreenPresentation, SymbolFamily, Symbols,
    TerminalProfile, ThemeFamily, ThemeId, ThemeRegistry, ThemeState, ThemeVariant, UiEvent,
    UiFrame, resolve_theme,
};
use ratatui::{Terminal, backend::TestBackend, layout::Rect, widgets::Paragraph};

#[derive(Clone, Debug, PartialEq, Eq)]
enum Action {
    Activate,
    Underlay,
    Overlay,
}

struct LayerScreen {
    label: &'static str,
    hit_area: Rect,
    action: Action,
    pointer_hit: Option<Arc<Mutex<Option<Action>>>>,
}

impl Screen<Action> for LayerScreen {
    fn render(&mut self, ui: &mut UiFrame<'_, '_, Action>, area: Rect) {
        ui.frame().render_widget(Paragraph::new(self.label), area);
        ui.hit_map().register(self.hit_area, self.action.clone());
    }

    fn handle_event(&mut self, event: &UiEvent<Action>) -> Interaction<Action> {
        if let UiEvent::Pointer { hit, .. } = event
            && let Some(observed) = &self.pointer_hit
        {
            *observed
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = hit.clone();
            return Interaction::unchanged();
        }
        Interaction::Unhandled
    }
}

fn layer(label: &'static str, hit_area: Rect, action: Action) -> LayerScreen {
    LayerScreen {
        label,
        hit_area,
        action,
        pointer_hit: None,
    }
}

fn test_theme() -> (TerminalProfile, ratatool::ResolvedTheme) {
    let profile = TerminalProfile::plain_ascii();
    let family = ThemeFamily::new(
        ThemeId::new("test"),
        ThemeVariant::default(),
        ThemeVariant::default(),
        ThemeVariant::default(),
        ThemeVariant::default(),
    );
    let theme = resolve_theme(&family, profile);
    (profile, theme)
}

fn mouse(kind: MouseEventKind, column: u16, row: u16) -> UiEvent<Action> {
    UiEvent::Mouse(MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    })
}

struct ButtonScreen;

impl Screen<Action> for ButtonScreen {
    fn render(&mut self, ui: &mut UiFrame<'_, '_, Action>, area: Rect) {
        ui.button(
            Rect::new(area.x, area.y, area.width, 1),
            &Button::new("Open").focused(true),
            Some(Action::Activate),
        );
    }

    fn handle_event(&mut self, _event: &UiEvent<Action>) -> Interaction<Action> {
        Interaction::Unhandled
    }
}

struct TickScreen {
    ticks: Arc<Mutex<usize>>,
    render_delay: Duration,
}

impl Screen<Action> for TickScreen {
    fn render(&mut self, _ui: &mut UiFrame<'_, '_, Action>, _area: Rect) {
        thread::sleep(self.render_delay);
    }

    fn handle_event(&mut self, event: &UiEvent<Action>) -> Interaction<Action> {
        if matches!(event, UiEvent::Tick) {
            let mut ticks = self
                .ticks
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            *ticks += 1;
        }
        Interaction::Unhandled
    }
}

#[test]
fn replacement_screen_hides_and_disables_the_previous_screen() {
    let (profile, theme) = test_theme();
    let mut runtime = Runtime::new(layer("underlay", Rect::new(0, 0, 10, 2), Action::Underlay));
    runtime
        .screens_mut()
        .push(layer("replacement", Rect::new(0, 0, 1, 1), Action::Overlay));
    let mut terminal = Terminal::new(TestBackend::new(12, 2)).unwrap();
    runtime
        .draw_with_theme(&mut terminal, &theme, profile)
        .unwrap();

    assert_eq!(runtime.action_at(5, 1), None);
    assert_eq!(runtime.action_at(0, 0), Some(&Action::Overlay));
}

#[test]
fn overlay_keeps_underlying_hit_regions() {
    let (profile, theme) = test_theme();
    let mut runtime = Runtime::new(layer("underlay", Rect::new(0, 0, 10, 2), Action::Underlay));
    runtime.screens_mut().push_with_presentation(
        layer("overlay", Rect::new(0, 0, 1, 1), Action::Overlay),
        ScreenPresentation::Overlay,
    );
    let mut terminal = Terminal::new(TestBackend::new(12, 2)).unwrap();
    runtime
        .draw_with_theme(&mut terminal, &theme, profile)
        .unwrap();

    assert_eq!(runtime.action_at(5, 1), Some(&Action::Underlay));
    assert_eq!(runtime.action_at(0, 0), Some(&Action::Overlay));
}

#[test]
fn modal_overlay_discards_underlying_hit_regions() {
    let (profile, theme) = test_theme();
    let mut runtime = Runtime::new(layer("underlay", Rect::new(0, 0, 10, 2), Action::Underlay));
    runtime.screens_mut().push_with_presentation(
        layer("modal", Rect::new(0, 0, 1, 1), Action::Overlay),
        ScreenPresentation::ModalOverlay,
    );
    let mut terminal = Terminal::new(TestBackend::new(12, 2)).unwrap();
    runtime
        .draw_with_theme(&mut terminal, &theme, profile)
        .unwrap();

    assert_eq!(runtime.action_at(5, 1), None);
    assert_eq!(runtime.action_at(0, 0), Some(&Action::Overlay));
}

#[test]
fn wheel_event_includes_the_action_under_the_pointer() {
    let (profile, theme) = test_theme();
    let observed = Arc::new(Mutex::new(None));
    let mut screen = layer("logs", Rect::new(0, 0, 10, 2), Action::Underlay);
    screen.pointer_hit = Some(Arc::clone(&observed));
    let mut runtime = Runtime::new(screen);
    let mut terminal = Terminal::new(TestBackend::new(12, 2)).unwrap();
    runtime
        .draw_with_theme(&mut terminal, &theme, profile)
        .unwrap();

    assert_eq!(
        runtime.dispatch(mouse(MouseEventKind::ScrollDown, 5, 1)),
        None
    );
    assert_eq!(
        *observed
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()),
        Some(Action::Underlay)
    );
}

#[test]
fn unhandled_resize_requests_a_redraw() {
    let (profile, theme) = test_theme();
    let mut runtime = Runtime::new(ButtonScreen);
    let mut terminal = Terminal::new(TestBackend::new(12, 2)).unwrap();
    assert!(
        runtime
            .draw_if_due_with_theme(&mut terminal, &theme, profile)
            .unwrap()
    );
    assert!(!runtime.needs_redraw());

    assert_eq!(
        runtime.dispatch(UiEvent::Resize {
            width: 20,
            height: 4,
        }),
        None
    );
    assert!(runtime.needs_redraw());
}

#[test]
fn elapsed_deadline_is_not_consumed_by_drawing_before_its_tick() {
    let (profile, theme) = test_theme();
    let ticks = Arc::new(Mutex::new(0));
    let mut runtime = Runtime::new(TickScreen {
        ticks: Arc::clone(&ticks),
        render_delay: Duration::ZERO,
    });
    let mut terminal = Terminal::new(TestBackend::new(12, 2)).unwrap();
    runtime
        .draw_if_due_with_theme(&mut terminal, &theme, profile)
        .unwrap();

    let deadline = Instant::now() - Duration::from_millis(1);
    runtime.request_redraw_at(deadline);

    assert!(
        !runtime
            .draw_if_due_with_theme(&mut terminal, &theme, profile)
            .unwrap()
    );
    assert_eq!(runtime.next_redraw(), Some(deadline));
    assert_eq!(*ticks.lock().unwrap(), 0);

    runtime.dispatch(UiEvent::Tick);

    assert_eq!(*ticks.lock().unwrap(), 1);
    assert_eq!(runtime.next_redraw(), None);
    assert!(runtime.needs_redraw());
}

#[test]
fn deadline_elapsed_during_an_unrelated_frame_remains_pending_for_tick() {
    let (profile, theme) = test_theme();
    let ticks = Arc::new(Mutex::new(0));
    let mut runtime = Runtime::new(TickScreen {
        ticks: Arc::clone(&ticks),
        render_delay: Duration::from_millis(25),
    });
    let mut terminal = Terminal::new(TestBackend::new(12, 2)).unwrap();
    runtime
        .draw_if_due_with_theme(&mut terminal, &theme, profile)
        .unwrap();

    let deadline = Instant::now() + Duration::from_millis(5);
    runtime.request_redraw_at(deadline);
    runtime.request_redraw();
    assert!(
        runtime
            .draw_if_due_with_theme(&mut terminal, &theme, profile)
            .unwrap()
    );
    assert!(Instant::now() >= deadline);
    assert_eq!(runtime.next_redraw(), Some(deadline));

    runtime.dispatch(UiEvent::Tick);

    assert_eq!(*ticks.lock().unwrap(), 1);
    assert_eq!(runtime.next_redraw(), None);
}

#[test]
fn successive_ticks_acknowledge_deadlines_in_order() {
    let (profile, theme) = test_theme();
    let ticks = Arc::new(Mutex::new(0));
    let mut runtime = Runtime::new(TickScreen {
        ticks: Arc::clone(&ticks),
        render_delay: Duration::ZERO,
    });
    let mut terminal = Terminal::new(TestBackend::new(12, 2)).unwrap();
    runtime
        .draw_if_due_with_theme(&mut terminal, &theme, profile)
        .unwrap();

    let first = Instant::now() - Duration::from_millis(1);
    let second = Instant::now() + Duration::from_millis(20);
    runtime.request_redraw_at(second);
    runtime.request_redraw_at(first);
    runtime.dispatch(UiEvent::Tick);
    assert_eq!(runtime.next_redraw(), Some(second));

    runtime
        .draw_with_theme(&mut terminal, &theme, profile)
        .unwrap();
    thread::sleep(Duration::from_millis(25));
    runtime.dispatch(UiEvent::Tick);

    assert_eq!(*ticks.lock().unwrap(), 2);
    assert_eq!(runtime.next_redraw(), None);
    assert!(runtime.needs_redraw());
}

#[test]
fn runtime_renders_and_routes_registered_mouse_action() {
    let mut family = ThemeFamily::new(
        ThemeId::new("test"),
        ThemeVariant::default(),
        ThemeVariant::default(),
        ThemeVariant::default(),
        ThemeVariant::default(),
    );
    family.symbols = SymbolFamily {
        ascii: Symbols {
            focus_marker: "!",
            ..Symbols::ascii()
        },
        ..SymbolFamily::default()
    };
    let profile = TerminalProfile::plain_ascii();
    let theme = resolve_theme(&family, profile);
    let mut terminal = Terminal::new(TestBackend::new(12, 2)).unwrap();
    let mut runtime = Runtime::new(ButtonScreen);
    runtime
        .draw_with_theme(&mut terminal, &theme, profile)
        .unwrap();
    assert!(!runtime.needs_redraw());

    let rendered: String = terminal.backend().buffer().content[..12]
        .iter()
        .map(|cell| cell.symbol())
        .collect();
    assert_eq!(rendered, "   ! Open   ");
    assert_eq!(
        runtime.dispatch(UiEvent::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 4,
            row: 0,
            modifiers: KeyModifiers::NONE,
        })),
        Some(Action::Activate)
    );
}

#[test]
fn runtime_draw_keeps_resolved_theme_and_profile_together() {
    let mut family = ThemeFamily::new(
        ThemeId::new("stateful"),
        ThemeVariant::default(),
        ThemeVariant::default(),
        ThemeVariant::default(),
        ThemeVariant::default(),
    );
    family.symbols = SymbolFamily {
        unicode: Symbols {
            focus_marker: "U",
            ..Symbols::unicode()
        },
        ascii: Symbols {
            focus_marker: "A",
            ..Symbols::ascii()
        },
    };
    let mut registry = ThemeRegistry::new();
    registry.register(family);
    let mut theme = ThemeState::new(
        ThemeId::new("stateful"),
        TerminalProfile::plain_ascii(),
        &registry,
    )
    .unwrap();
    let mut runtime = Runtime::new(ButtonScreen);
    let mut terminal = Terminal::new(TestBackend::new(12, 1)).unwrap();

    runtime.draw(&mut terminal, &theme).unwrap();
    let ascii: String = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect();
    assert!(ascii.contains('A'));

    theme
        .set_profile(TerminalProfile::truecolor_unicode(), &registry)
        .unwrap();
    runtime.draw(&mut terminal, &theme).unwrap();
    let unicode: String = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect();
    assert!(unicode.contains('U'));
}
