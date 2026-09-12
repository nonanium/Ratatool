use std::io::{self, Stdout, Write};

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionOptions {
    pub raw_mode: bool,
    pub alternate_screen: bool,
    pub mouse_capture: bool,
    pub hide_cursor: bool,
}

impl Default for SessionOptions {
    fn default() -> Self {
        Self {
            raw_mode: true,
            alternate_screen: true,
            mouse_capture: false,
            hide_cursor: true,
        }
    }
}

pub struct TerminalSession<W: Write = Stdout> {
    terminal: Terminal<CrosstermBackend<W>>,
    active: SessionOptions,
}

impl TerminalSession<Stdout> {
    pub fn enter(options: SessionOptions) -> io::Result<Self> {
        let mut stdout = io::stdout();
        if options.raw_mode {
            enable_raw_mode()?;
        }
        if options.alternate_screen
            && let Err(error) = execute!(stdout, EnterAlternateScreen)
        {
            if options.raw_mode {
                let _ = disable_raw_mode();
            }
            return Err(error);
        }
        if options.mouse_capture
            && let Err(error) = execute!(stdout, EnableMouseCapture)
        {
            if options.alternate_screen {
                let _ = execute!(stdout, LeaveAlternateScreen);
            }
            if options.raw_mode {
                let _ = disable_raw_mode();
            }
            return Err(error);
        }

        let backend = CrosstermBackend::new(stdout);
        let terminal = match Terminal::new(backend) {
            Ok(terminal) => terminal,
            Err(error) => {
                let mut stdout = io::stdout();
                if options.mouse_capture {
                    let _ = execute!(stdout, DisableMouseCapture);
                }
                if options.alternate_screen {
                    let _ = execute!(stdout, LeaveAlternateScreen);
                }
                if options.raw_mode {
                    let _ = disable_raw_mode();
                }
                return Err(error);
            }
        };
        let mut session = Self {
            terminal,
            active: options,
        };
        if options.hide_cursor
            && let Err(error) = session.terminal.hide_cursor()
        {
            let _ = session.restore();
            return Err(error);
        }
        Ok(session)
    }
}

impl<W: Write> TerminalSession<W> {
    pub fn terminal(&self) -> &Terminal<CrosstermBackend<W>> {
        &self.terminal
    }

    pub fn terminal_mut(&mut self) -> &mut Terminal<CrosstermBackend<W>> {
        &mut self.terminal
    }

    pub fn restore(&mut self) -> io::Result<()> {
        let mut first_error = None;
        if self.active.hide_cursor {
            let result = self.terminal.show_cursor();
            record_restoration(&mut self.active.hide_cursor, result, &mut first_error);
        }
        if self.active.mouse_capture {
            let result = execute!(self.terminal.backend_mut(), DisableMouseCapture);
            record_restoration(&mut self.active.mouse_capture, result, &mut first_error);
        }
        if self.active.alternate_screen {
            let result = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
            record_restoration(&mut self.active.alternate_screen, result, &mut first_error);
        }
        if self.active.raw_mode {
            let result = disable_raw_mode();
            record_restoration(&mut self.active.raw_mode, result, &mut first_error);
        }
        first_error.map_or(Ok(()), Err)
    }
}

fn record_restoration(
    active: &mut bool,
    result: io::Result<()>,
    first_error: &mut Option<io::Error>,
) {
    match result {
        Ok(()) => *active = false,
        Err(error) if first_error.is_none() => *first_error = Some(error),
        Err(_) => {}
    }
}

impl<W: Write> Drop for TerminalSession<W> {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_restoration_remains_active_for_retry() {
        let mut active = true;
        let mut first_error = None;
        record_restoration(
            &mut active,
            Err(io::Error::other("temporary failure")),
            &mut first_error,
        );

        assert!(active);
        assert_eq!(first_error.unwrap().kind(), io::ErrorKind::Other);

        let mut retry_error = None;
        record_restoration(&mut active, Ok(()), &mut retry_error);
        assert!(!active);
        assert!(retry_error.is_none());
    }
}
