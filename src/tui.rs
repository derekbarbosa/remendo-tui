//! Terminal user interface lifecycle management.
//!
//! The `Tui` struct wraps the ratatui terminal, event channels,
//! and async event handler task. It manages raw mode, alternate
//! screen, panic hooks, and clean shutdown.

use crate::event::Event;
use color_eyre::Result;
use crossterm::event::{
    DisableBracketedPaste, DisableFocusChange, DisableMouseCapture, EnableBracketedPaste,
    EnableFocusChange, EnableMouseCapture, EventStream,
};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use futures::StreamExt;
use ratatui::DefaultTerminal;
use std::io::{self, stdout};
use std::panic;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

/// Terminal UI wrapper managing lifecycle and event delivery.
///
/// The lifecycle is split into two phases:
/// 1. `Tui::new()` — creates channels and config; no terminal side effects
/// 2. `Tui::enter()` — initializes the terminal, enters raw mode, spawns event task
///
/// This split ensures config loading and stderr output happen before
/// the terminal is taken over.
pub struct Tui {
    /// The ratatui terminal instance (initialized in `enter()`).
    terminal: Option<DefaultTerminal>,
    /// Receiver for events from the event handler task.
    event_rx: mpsc::UnboundedReceiver<Event>,
    /// Sender for events (cloned into the event task).
    event_tx: mpsc::UnboundedSender<Event>,
    /// Handle to the spawned event handler task.
    task: Option<JoinHandle<()>>,
    /// Cancellation token to stop the event handler task.
    cancellation_token: CancellationToken,
    /// Target tick rate (ticks per second).
    tick_rate: f64,
    /// Target frame rate (frames per second).
    frame_rate: f64,
}

impl Tui {
    /// Create a new TUI instance **without** initializing the terminal.
    ///
    /// No side effects — the terminal remains in normal mode.
    /// Call [`Tui::enter`] to activate raw mode and start the event
    /// handler task.
    #[must_use]
    pub fn new(tick_rate: f64, frame_rate: f64) -> Self {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        Self {
            terminal: None,
            event_rx,
            event_tx,
            task: None,
            cancellation_token: CancellationToken::new(),
            tick_rate,
            frame_rate,
        }
    }

    /// Initialize the terminal and enter raw mode.
    ///
    /// This enables raw mode, alternate screen, mouse capture,
    /// focus change events, and bracketed paste. It also installs
    /// a panic hook that restores the terminal before displaying
    /// the error report, and spawns the async event handler task.
    ///
    /// # Errors
    ///
    /// Returns an error if terminal setup fails (instead of panicking).
    pub fn enter(&mut self) -> Result<()> {
        terminal::enable_raw_mode()?;
        crossterm::execute!(
            stdout(),
            EnterAlternateScreen,
            EnableMouseCapture,
            EnableFocusChange,
            EnableBracketedPaste,
        )?;

        // Initialize the terminal after entering raw mode
        let backend = ratatui::prelude::CrosstermBackend::new(stdout());
        let terminal = ratatui::Terminal::new(backend)?;
        self.terminal = Some(terminal);

        // Install panic hook that restores the terminal
        let original_hook = panic::take_hook();
        panic::set_hook(Box::new(move |panic_info| {
            let _ = Self::reset();
            original_hook(panic_info);
        }));

        // Spawn the event handler task
        let event_tx = self.event_tx.clone();
        let cancellation_token = self.cancellation_token.clone();
        let tick_delay = Duration::from_secs_f64(1.0 / self.tick_rate);
        let render_delay = Duration::from_secs_f64(1.0 / self.frame_rate);

        self.task = Some(tokio::spawn(async move {
            let mut reader = EventStream::new();
            let mut tick_interval = tokio::time::interval(tick_delay);
            let mut render_interval = tokio::time::interval(render_delay);

            // Send Init event
            let _ = event_tx.send(Event::Init);

            loop {
                #[allow(clippy::ignored_unit_patterns)]
                let event = tokio::select! {
                    () = cancellation_token.cancelled() => break,
                    _ = tick_interval.tick() => Event::Tick,
                    _ = render_interval.tick() => Event::Render,
                    crossterm_event = reader.next() => {
                        match crossterm_event {
                            Some(Ok(evt)) => translate_crossterm_event(evt),
                            Some(Err(_)) => Event::Error,
                            None => break,
                        }
                    }
                };

                if event_tx.send(event).is_err() {
                    // Receiver dropped — app is shutting down
                    break;
                }
            }
        }));

        Ok(())
    }

    /// Wait for the next event from the event handler task.
    ///
    /// # Errors
    ///
    /// Returns an error if the event channel is closed unexpectedly.
    pub async fn next(&mut self) -> Result<Event> {
        self.event_rx
            .recv()
            .await
            .ok_or_else(|| color_eyre::eyre::eyre!("event channel closed"))
    }

    /// Exit raw mode and restore the terminal.
    ///
    /// Cancels the event handler task and restores the terminal state.
    ///
    /// # Errors
    ///
    /// Returns an error if terminal restoration fails.
    pub fn exit(&mut self) -> Result<()> {
        self.cancellation_token.cancel();
        self.task = None;
        self.terminal = None;
        Self::reset()?;
        Ok(())
    }

    /// Draw a frame using the provided rendering closure.
    ///
    /// # Errors
    ///
    /// Returns an error if the terminal is not initialized or drawing fails.
    pub fn draw(&mut self, f: impl FnOnce(&mut ratatui::Frame)) -> Result<()> {
        let terminal = self.terminal.as_mut().ok_or_else(|| {
            color_eyre::eyre::eyre!("terminal not initialized — call enter() first")
        })?;
        terminal.draw(f)?;
        Ok(())
    }

    /// Temporarily suspend the TUI for an external process (e.g., editor).
    ///
    /// Leaves alternate screen and disables raw mode so the subprocess
    /// can interact with the terminal normally. Call `resume()` after
    /// the subprocess exits.
    ///
    /// # Errors
    ///
    /// Returns an error if terminal state cannot be changed.
    pub fn suspend(&mut self) -> Result<()> {
        Self::reset()?;
        Ok(())
    }

    /// Resume the TUI after a `suspend()` call.
    ///
    /// Re-enters raw mode and alternate screen. The event handler
    /// task is still running — it will resume delivering events.
    ///
    /// # Errors
    ///
    /// Returns an error if terminal state cannot be restored.
    pub fn resume(&mut self) -> Result<()> {
        self.drain_events();
        terminal::enable_raw_mode()?;
        crossterm::execute!(
            stdout(),
            EnterAlternateScreen,
            EnableMouseCapture,
            EnableFocusChange,
            EnableBracketedPaste,
        )?;
        // Re-create the terminal backend for the new stdout
        let backend = ratatui::prelude::CrosstermBackend::new(stdout());
        self.terminal = Some(ratatui::Terminal::new(backend)?);
        Ok(())
    }

    /// Discard all queued events from the event channel.
    ///
    /// Called by [`resume`](Self::resume) to prevent stale input that
    /// accumulated during a [`suspend`](Self::suspend) from being
    /// interpreted as TUI commands. The background event handler task
    /// keeps running during suspend, so its `EventStream` can race
    /// with the child process for stdin and capture keypresses (e.g.,
    /// `q` to quit an editor) that would otherwise cause unintended
    /// actions like quitting the app.
    fn drain_events(&mut self) {
        while self.event_rx.try_recv().is_ok() {}
    }

    /// Reset terminal state (used by both exit and panic hook).
    fn reset() -> io::Result<()> {
        terminal::disable_raw_mode()?;
        crossterm::execute!(
            stdout(),
            DisableMouseCapture,
            DisableFocusChange,
            DisableBracketedPaste,
            LeaveAlternateScreen,
        )?;
        Ok(())
    }
}

impl Drop for Tui {
    fn drop(&mut self) {
        self.cancellation_token.cancel();
        let _ = Self::reset();
    }
}

/// Translate a crossterm event into our application `Event` enum.
fn translate_crossterm_event(event: crossterm::event::Event) -> Event {
    match event {
        crossterm::event::Event::Key(key) => Event::Key(key),
        crossterm::event::Event::Mouse(mouse) => Event::Mouse(mouse),
        crossterm::event::Event::Resize(w, h) => Event::Resize(w, h),
        crossterm::event::Event::FocusGained => Event::FocusGained,
        crossterm::event::Event::FocusLost => Event::FocusLost,
        crossterm::event::Event::Paste(text) => Event::Paste(text),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn drain_events_empties_channel() {
        let mut tui = Tui::new(1.0, 1.0);
        // Inject several events via the sender (simulating the background task)
        tui.event_tx.send(Event::Tick).unwrap();
        tui.event_tx.send(Event::Render).unwrap();
        tui.event_tx
            .send(Event::Key(crossterm::event::KeyEvent::new(
                crossterm::event::KeyCode::Char('q'),
                crossterm::event::KeyModifiers::NONE,
            )))
            .unwrap();

        tui.drain_events();

        // Channel should be empty
        assert!(tui.event_rx.try_recv().is_err());
    }

    #[test]
    fn drain_events_on_empty_channel_is_noop() {
        let mut tui = Tui::new(1.0, 1.0);
        tui.drain_events();
        assert!(tui.event_rx.try_recv().is_err());
    }
}
