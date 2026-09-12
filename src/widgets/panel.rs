use std::borrow::Cow;

use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    symbols::border,
    text::Line,
    widgets::{Block, Borders, Paragraph},
};

pub use ratatui::widgets::Padding;

use crate::theme::{PanelPresentation, ResolvedTheme};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PanelKind {
    #[default]
    Normal,
    Overlay,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Panel<'a> {
    pub title: Option<Cow<'a, str>>,
    pub footer: Option<Cow<'a, str>>,
    pub focused: bool,
    pub enabled: bool,
    pub padding: Padding,
    pub kind: PanelKind,
}

impl Default for Panel<'_> {
    fn default() -> Self {
        Self {
            title: None,
            footer: None,
            focused: false,
            enabled: true,
            padding: Padding::ZERO,
            kind: PanelKind::Normal,
        }
    }
}

impl<'a> Panel<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn title(mut self, title: impl Into<Cow<'a, str>>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn footer(mut self, footer: impl Into<Cow<'a, str>>) -> Self {
        self.footer = Some(footer.into());
        self
    }

    pub const fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub const fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub const fn padding(mut self, padding: u16) -> Self {
        self.padding = Padding::uniform(padding);
        self
    }

    pub const fn insets(mut self, padding: Padding) -> Self {
        self.padding = padding;
        self
    }

    pub const fn kind(mut self, kind: PanelKind) -> Self {
        self.kind = kind;
        self
    }
}

pub fn render_panel(
    frame: &mut Frame<'_>,
    area: Rect,
    panel: &Panel<'_>,
    theme: &ResolvedTheme,
) -> Rect {
    match theme.metrics.panel_presentation {
        PanelPresentation::Bordered => render_bordered_panel(frame, area, panel, theme),
        PanelPresentation::Surface => render_surface_panel(frame, area, panel, theme),
    }
}

fn render_bordered_panel(
    frame: &mut Frame<'_>,
    area: Rect,
    panel: &Panel<'_>,
    theme: &ResolvedTheme,
) -> Rect {
    let border_style = if !panel.enabled {
        theme.borders.disabled
    } else if panel.focused {
        theme.borders.focused
    } else {
        theme.borders.normal
    };
    let symbols = theme.symbols.borders;
    let border_set = border::Set {
        top_left: symbols.top_left,
        top_right: symbols.top_right,
        bottom_left: symbols.bottom_left,
        bottom_right: symbols.bottom_right,
        vertical_left: symbols.vertical,
        vertical_right: symbols.vertical,
        horizontal_top: symbols.horizontal,
        horizontal_bottom: symbols.horizontal,
    };
    let mut block = Block::new()
        .borders(Borders::ALL)
        .border_set(border_set)
        .border_style(border_style)
        .style(panel_base_style(panel, theme))
        .padding(panel.padding);
    if let Some(title) = panel.title.as_deref() {
        block = block.title_top(title);
    }
    if let Some(footer) = panel.footer.as_deref() {
        block = block.title_bottom(footer);
    }
    let inner = block.inner(area);
    frame.render_widget(block, area);
    inner
}

fn render_surface_panel(
    frame: &mut Frame<'_>,
    area: Rect,
    panel: &Panel<'_>,
    theme: &ResolvedTheme,
) -> Rect {
    let panel_style = if !panel.enabled {
        panel_base_style(panel, theme).patch(theme.borders.disabled)
    } else if panel.focused && panel.kind == PanelKind::Normal {
        theme.surfaces.panel.patch(theme.surfaces.panel_focused)
    } else {
        panel_base_style(panel, theme)
    };
    frame.render_widget(Block::new().style(panel_style), area);

    let title_height = u16::from(panel.title.is_some() && area.height > 0);
    let footer_height =
        u16::from(panel.footer.is_some() && area.height.saturating_sub(title_height) > 0);
    if let Some(title) = panel.title.as_deref()
        && title_height > 0
    {
        let style = panel_style
            .patch(theme.surfaces.panel_header)
            .patch(theme.text.heading);
        let header_area = Rect::new(area.x, area.y, area.width, 1);
        frame.render_widget(Block::new().style(style), header_area);
        frame.render_widget(
            Paragraph::new(Line::from(title)).style(style),
            inset_horizontally(header_area, panel.padding),
        );
    }
    if let Some(footer) = panel.footer.as_deref()
        && footer_height > 0
    {
        let style = panel_style.patch(theme.text.muted);
        frame.render_widget(
            Paragraph::new(Line::from(footer)).style(style),
            inset_horizontally(
                Rect::new(area.x, area.bottom().saturating_sub(1), area.width, 1),
                panel.padding,
            ),
        );
    }

    let horizontal_padding = panel
        .padding
        .left
        .saturating_add(panel.padding.right)
        .min(area.width);
    let reserved_height = title_height
        .saturating_add(footer_height)
        .saturating_add(panel.padding.top)
        .saturating_add(panel.padding.bottom)
        .min(area.height);
    Rect::new(
        area.x.saturating_add(panel.padding.left.min(area.width)),
        area.y.saturating_add(title_height).saturating_add(
            panel
                .padding
                .top
                .min(area.height.saturating_sub(title_height)),
        ),
        area.width.saturating_sub(horizontal_padding),
        area.height.saturating_sub(reserved_height),
    )
}

fn panel_base_style(panel: &Panel<'_>, theme: &ResolvedTheme) -> Style {
    match panel.kind {
        PanelKind::Normal => theme.surfaces.panel,
        PanelKind::Overlay => theme.surfaces.overlay,
    }
}

fn inset_horizontally(area: Rect, padding: Padding) -> Rect {
    let horizontal = padding.left.saturating_add(padding.right).min(area.width);
    Rect::new(
        area.x.saturating_add(padding.left.min(area.width)),
        area.y,
        area.width.saturating_sub(horizontal),
        area.height,
    )
}

#[cfg(test)]
mod tests {
    use ratatui::{
        Terminal,
        backend::TestBackend,
        style::{Color, Style},
    };

    use super::*;
    use crate::{ResolvedTheme, Symbols, ThemeVariant};

    #[test]
    fn panel_uses_resolved_borders_and_returns_padded_inner_area() {
        let variant = ThemeVariant::default();
        let theme = ResolvedTheme {
            text: variant.text,
            borders: variant.borders,
            surfaces: variant.surfaces,
            buttons: variant.buttons,
            status: variant.status,
            input: variant.input,
            selection: variant.selection,
            metrics: variant.metrics,
            symbols: Symbols::ascii(),
        };
        let mut inner = Rect::default();
        let mut terminal = Terminal::new(TestBackend::new(10, 5)).unwrap();
        terminal
            .draw(|frame| {
                inner = render_panel(
                    frame,
                    frame.area(),
                    &Panel::new().title("Title").padding(1),
                    &theme,
                );
            })
            .unwrap();
        assert_eq!(inner, Rect::new(2, 2, 6, 1));
        assert_eq!(terminal.backend().buffer()[(0, 0)].symbol(), "+");
    }

    #[test]
    fn surface_panel_uses_header_and_footer_rows_without_a_border() {
        let mut variant = ThemeVariant::default();
        variant.metrics.panel_presentation = PanelPresentation::Surface;
        variant.surfaces.panel = Style::default().bg(Color::Blue);
        variant.surfaces.panel_header = Style::default().bg(Color::Red);
        let theme = ResolvedTheme {
            text: variant.text,
            borders: variant.borders,
            surfaces: variant.surfaces,
            buttons: variant.buttons,
            status: variant.status,
            input: variant.input,
            selection: variant.selection,
            metrics: variant.metrics,
            symbols: Symbols::ascii(),
        };
        let mut inner = Rect::default();
        let mut terminal = Terminal::new(TestBackend::new(10, 6)).unwrap();
        terminal
            .draw(|frame| {
                inner = render_panel(
                    frame,
                    frame.area(),
                    &Panel::new().title("Header").footer("Footer").padding(1),
                    &theme,
                );
            })
            .unwrap();

        assert_eq!(inner, Rect::new(1, 2, 8, 2));
        assert_eq!(terminal.backend().buffer()[(1, 0)].symbol(), "H");
        assert_eq!(terminal.backend().buffer()[(1, 5)].symbol(), "F");
        assert_eq!(terminal.backend().buffer()[(9, 3)].bg, Color::Blue);
        assert_eq!(terminal.backend().buffer()[(9, 0)].bg, Color::Red);
    }

    #[test]
    fn surface_panel_uses_focused_style_and_asymmetric_insets() {
        let mut variant = ThemeVariant::default();
        variant.metrics.panel_presentation = PanelPresentation::Surface;
        variant.surfaces.panel = Style::default().bg(Color::Blue);
        variant.surfaces.panel_focused = Style::default().bg(Color::Green);
        let theme = ResolvedTheme {
            text: variant.text,
            borders: variant.borders,
            surfaces: variant.surfaces,
            buttons: variant.buttons,
            status: variant.status,
            input: variant.input,
            selection: variant.selection,
            metrics: variant.metrics,
            symbols: Symbols::ascii(),
        };
        let mut inner = Rect::default();
        let mut terminal = Terminal::new(TestBackend::new(10, 5)).unwrap();
        terminal
            .draw(|frame| {
                inner = render_panel(
                    frame,
                    frame.area(),
                    &Panel::new()
                        .title("Header")
                        .focused(true)
                        .insets(Padding::new(1, 2, 0, 1)),
                    &theme,
                );
            })
            .unwrap();

        assert_eq!(inner, Rect::new(1, 1, 7, 3));
        assert_eq!(terminal.backend().buffer()[(9, 4)].bg, Color::Green);
    }
}
