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
pub struct Tui {
    /// The ratatui terminal instance.
    pub terminal: DefaultTerminal,
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
    /// Create a new TUI instance without entering raw mode.
    ///
    /// Call [`Tui::enter`] to activate the terminal and start
    /// the event handler task.
    ///
    /// # Errors
    ///
    /// Returns an error if the terminal cannot be initialized.
    pub fn new(tick_rate: f64, frame_rate: f64) -> Result<Self> {
        let terminal = ratatui::init();
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        Ok(Self {
            terminal,
            event_rx,
            event_tx,
            task: None,
            cancellation_token: CancellationToken::new(),
            tick_rate,
            frame_rate,
        })
    }

    /// Enter raw mode and start the async event handler task.
    ///
    /// Enables raw mode, alternate screen, mouse capture,
    /// focus change events, and bracketed paste. Installs a
    /// panic hook that restores the terminal before displaying
    /// the error report. Spawns a tokio task that polls crossterm
    /// events and tick/render intervals.
    ///
    /// # Errors
    ///
    /// Returns an error if terminal setup fails.
    pub fn enter(&mut self) -> Result<()> {
        terminal::enable_raw_mode()?;
        crossterm::execute!(
            stdout(),
            EnterAlternateScreen,
            EnableMouseCapture,
            EnableFocusChange,
            EnableBracketedPaste,
        )?;

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
        // The task will stop on its own when the token is cancelled
        self.task = None;
        Self::reset()?;
        Ok(())
    }

    /// Draw a frame using the provided rendering closure.
    ///
    /// # Errors
    ///
    /// Returns an error if drawing fails.
    pub fn draw(&mut self, f: impl FnOnce(&mut ratatui::Frame)) -> Result<()> {
        self.terminal.draw(f)?;
        Ok(())
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
