# Ratatool

Ratatool is an adaptive terminal UI toolkit built on [Ratatui](https://ratatui.rs/). It began as the reusable UI layer for Iota, while keeping Ratatui's `Frame`, widgets, styles, and backends available as escape hatches.

The library separates terminal capability detection, semantic themes, stateful controls, rendering, and application runtime concerns. A theme resolves once for a `TerminalProfile`; widgets then consume the resulting styles and symbols without repeating capability checks.

```rust
use ratatool::{
    TerminalOptions, ThemeFamily, ThemeId, ThemeVariant, detect_current, resolve_theme,
};

let profile = detect_current(&TerminalOptions::default());
let family = ThemeFamily::new(
    ThemeId::new("default"),
    ThemeVariant::default(),
    ThemeVariant::default(),
    ThemeVariant::default(),
    ThemeVariant::default(),
);
let theme = resolve_theme(&family, profile);
```

`TerminalOptions::overrides` can replace the exact detected color depth, charset, or normal interactive-terminal modifier assumption. Deterministic constructors such as `TerminalProfile::plain_ascii()` and `TerminalProfile::ansi256_unicode()` are useful for tests and command-line overrides.

Themes own both Unicode and ASCII symbol sets. RGB colors automatically quantize for ANSI256 and ANSI16 terminals; `resolve_theme_with_fallback` can instead reject or reset unrepresentable colors. State styles such as text-input selection are overlays and are patched onto the base control style.

The framework layer provides `TerminalSession`, `UiEvent<M>`, `Runtime<M>`, `ScreenStack<M>`, `FocusManager`, `FocusGraph`, `HitMap<M>`, and redraw deadlines. Application messages remain generic. Terminal key presses and repeats are delivered; release events are discarded at the framework boundary.

`Screen<M>` is `Send`, allowing a runtime and its screens to be owned by a regular Tokio task. `ScreenStack::push()` uses replacement presentation, so only the new screen renders. `push_with_presentation()` accepts two additional layer types. `Overlay` renders lower screens and keeps their mouse hit regions active. `ModalOverlay` renders lower screens and discards their hit regions before rendering the modal. Event handling always belongs to the top screen. Boxed screens can be pushed directly, while `replace`, `pop_to_root`, and `clear` manage navigation history.

Panels support asymmetric `Padding`, independent normal, focused, and header surface styles, and `PanelKind::Overlay`. Dialogs use the overlay surface and stack their buttons vertically when horizontal space is insufficient. `TextInputState::insert_text` replaces the current selection and provides the common paste operation for focused fields.

Runtime hit testing enriches raw mouse input into `UiEvent::Pointer { event, hit }` before screen dispatch. Left button presses on a hit region still return its action automatically. Other pointer input, including wheel events, reaches the top screen with the hit action attached. `request_redraw()` requests a frame; `request_redraw_at()` schedules a timer wakeup. The event source turns that wakeup into `UiEvent::Tick`, and runtime acknowledges due deadlines while dispatching the tick before rendering the next frame.

Enable the optional `tokio` feature for `AsyncEventSource<M>`, which merges Crossterm input, a Tokio message channel, and redraw deadlines. The runtime remains independent of Tokio. A complete asynchronous driver has this shape:

```rust,ignore
while runtime.is_running() {
    runtime.draw_if_due(
        session.terminal_mut(),
        &theme_state,
    )?;

    let Some(event) = source.next(runtime.next_redraw()).await? else {
        break;
    };
    if let Some(message) = runtime.dispatch(event) {
        app.handle_message(message, &mut runtime, &mut theme_state).await?;
    }
}
```

`Runtime` owns resize invalidation. Screens may return `Interaction::Unhandled` for a resize and the next loop iteration still redraws the terminal. Timed requests remain pending until their `UiEvent::Tick` is dispatched. A tick acknowledges all deadlines due at the time it is dispatched while preserving later deadlines. Closing the application-message channel disables that input branch without terminating terminal input.

The preferred draw and synchronous run APIs take `ThemeState`, which keeps the resolved theme and terminal profile together. Lower-level `*_with_theme` methods remain available for applications that do not use a theme registry.

For Iota, the intended boundary is one Ratatool root screen named `IotaShell`. That shell owns Iota's content-screen stack, header, footer and key hints, notifications, and global shortcuts. Iota-specific screens keep their own trait and metadata. Iota's theme should wrap the generic theme rather than extending Ratatool with application fields:

```rust,ignore
struct IotaResolvedTheme {
    console: ConsoleTheme,
    logs: LogTheme,
    graphs: GraphTheme,
    markdown: MarkdownTheme,
}

struct IotaThemeState {
    ui: ratatool::ThemeState,
    iota: IotaResolvedTheme,
}
```

Run the standalone demo with:

```text
cargo run --example demo
```

Press `q` or Escape to exit. The session guard restores raw mode, mouse capture, cursor visibility, and the alternate screen on normal return and unwinding.

Run the integration showcase with:

```text
cargo run --example showcase --features tokio
```

The showcase combines header controls, menus, radio and checkbox groups, text input, mouse wheel routing, screen replacement, a modal dialog, notification deadlines, panel presentation, theme switching, and terminal profile switching.

## Validation

```text
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo doc --no-deps --all-features
```

CI runs these checks on the declared Rust 1.88 minimum and on stable Rust.

Ratatool is licensed under MIT or Apache-2.0.
