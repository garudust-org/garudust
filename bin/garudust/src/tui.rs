use std::io;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Terminal,
};
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum AgentEvent {
    #[allow(dead_code)]
    Output(String),
    OutputChunk(String),
    Thinking,
    Done {
        iterations: u32,
        input_tokens: u32,
        output_tokens: u32,
    },
    Error(String),
}

#[derive(Debug, Clone)]
pub enum TuiEvent {
    Submit(String),
    Quit,
    NewSession,
    ChangeModel(String),
}

pub struct Tui {
    input: String,
    messages: Vec<(Role, String)>,
    status: String,
    scroll: u16,
    streaming: bool,
}

#[derive(Clone)]
enum Role {
    User,
    Assistant,
    Error,
}

impl Tui {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            messages: Vec::new(),
            status: "Ready — press Enter to send, Ctrl+C to quit".into(),
            scroll: 0,
            streaming: false,
        }
    }

    pub async fn run(
        tx_event: mpsc::Sender<TuiEvent>,
        mut rx_agent: mpsc::Receiver<AgentEvent>,
    ) -> io::Result<()> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut term = Terminal::new(backend)?;

        let mut tui = Tui::new();
        tui.messages.push((
            Role::Assistant,
            "Garudust — type your task and press Enter.".into(),
        ));

        loop {
            // Drain agent events (non-blocking)
            while let Ok(ev) = rx_agent.try_recv() {
                tui.handle_agent_event(ev);
            }

            term.draw(|f| tui.render(f))?;

            // Poll keyboard (50 ms timeout so agent events render promptly)
            if event::poll(std::time::Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    match (key.code, key.modifiers) {
                        (KeyCode::Char('c' | 'q'), KeyModifiers::CONTROL) => {
                            let _ = tx_event.send(TuiEvent::Quit).await;
                            break;
                        }
                        (KeyCode::Enter, _) => {
                            let text = tui.input.trim().to_string();
                            if !text.is_empty() {
                                tui.input.clear();
                                if let Some(rest) = text.strip_prefix('/') {
                                    let (cmd, args) = rest
                                        .split_once(' ')
                                        .map_or((rest, None), |(c, a)| (c, Some(a.trim())));
                                    match cmd {
                                        "new" => {
                                            tui.messages.clear();
                                            tui.messages.push((
                                                Role::Assistant,
                                                "New session started.".into(),
                                            ));
                                            let _ = tx_event.send(TuiEvent::NewSession).await;
                                        }
                                        "model" => match args {
                                            Some(m) if !m.is_empty() => {
                                                tui.messages.push((
                                                    Role::Assistant,
                                                    format!("Model → {m}"),
                                                ));
                                                let _ = tx_event
                                                    .send(TuiEvent::ChangeModel(m.to_string()))
                                                    .await;
                                            }
                                            _ => tui.messages.push((
                                                Role::Error,
                                                "Usage: /model <model-name>".into(),
                                            )),
                                        },
                                        "help" => {
                                            tui.messages.push((
                                                Role::Assistant,
                                                "/new       — clear history and start fresh\n\
                                                 /model <n> — switch to a different model\n\
                                                 /help      — show this help"
                                                    .into(),
                                            ));
                                        }
                                        _ => {
                                            tui.messages.push((
                                                Role::Error,
                                                format!(
                                                    "Unknown command /{cmd}. Type /help for help."
                                                ),
                                            ));
                                        }
                                    }
                                } else {
                                    tui.messages.push((Role::User, text.clone()));
                                    tui.status = "Thinking…".into();
                                    let _ = tx_event.send(TuiEvent::Submit(text)).await;
                                }
                            }
                        }
                        (KeyCode::Backspace, _) => {
                            tui.input.pop();
                        }
                        (KeyCode::Up, _) => tui.scroll = tui.scroll.saturating_sub(1),
                        (KeyCode::Down, _) => tui.scroll = tui.scroll.saturating_add(1),
                        (KeyCode::Char(c), _) => tui.input.push(c),
                        _ => {}
                    }
                }
            }
        }

        disable_raw_mode()?;
        execute!(
            term.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        Ok(())
    }

    fn handle_agent_event(&mut self, ev: AgentEvent) {
        match ev {
            AgentEvent::Output(text) => {
                self.streaming = false;
                self.messages.push((Role::Assistant, text));
                self.status = "Ready".into();
                self.scroll = u16::MAX;
            }
            AgentEvent::OutputChunk(delta) => {
                if self.streaming {
                    if let Some((Role::Assistant, buf)) = self.messages.last_mut() {
                        buf.push_str(&delta);
                    }
                } else {
                    self.streaming = true;
                    self.messages.push((Role::Assistant, delta));
                }
                self.scroll = u16::MAX;
            }
            AgentEvent::Thinking => {
                self.streaming = false;
                self.status = "Thinking…".into();
            }
            AgentEvent::Done {
                iterations,
                input_tokens,
                output_tokens,
            } => {
                self.streaming = false;
                self.status = format!(
                    "Done — {iterations} iterations | {input_tokens} in / {output_tokens} out tokens"
                );
            }
            AgentEvent::Error(e) => {
                self.streaming = false;
                self.messages.push((Role::Error, format!("Error: {e}")));
                self.status = "Error — ready for next task".into();
            }
        }
    }

    fn render(&self, f: &mut ratatui::Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),
                Constraint::Length(1),
                Constraint::Length(3),
            ])
            .split(f.area());

        // ── Messages pane ──
        let lines: Vec<Line> = self
            .messages
            .iter()
            .flat_map(|(role, text)| {
                let (prefix, style) = match role {
                    Role::User => (
                        "You  › ",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Role::Assistant => ("  AI › ", Style::default().fg(Color::Green)),
                    Role::Error => ("  !! › ", Style::default().fg(Color::Red)),
                };
                text.lines()
                    .enumerate()
                    .map(move |(i, line)| {
                        if i == 0 {
                            Line::from(vec![
                                Span::styled(prefix, style),
                                Span::raw(line.to_string()),
                            ])
                        } else {
                            Line::from(vec![Span::raw("       "), Span::raw(line.to_string())])
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .collect();

        let total_lines = u16::try_from(lines.len()).unwrap_or(u16::MAX);
        let visible = chunks[0].height.saturating_sub(2);
        let scroll = if self.scroll == u16::MAX {
            total_lines.saturating_sub(visible)
        } else {
            self.scroll.min(total_lines.saturating_sub(visible))
        };

        let messages = Paragraph::new(Text::from(lines))
            .block(Block::default().borders(Borders::ALL).title(" Garudust "))
            .wrap(Wrap { trim: false })
            .scroll((scroll, 0));
        f.render_widget(messages, chunks[0]);

        // ── Status bar ──
        let status =
            Paragraph::new(self.status.as_str()).style(Style::default().fg(Color::DarkGray));
        f.render_widget(status, chunks[1]);

        // ── Input box ──
        let input = Paragraph::new(self.input.as_str())
            .block(Block::default().borders(Borders::ALL).title(" Input "))
            .style(Style::default().fg(Color::White));
        f.render_widget(input, chunks[2]);

        // Show cursor inside input box
        let input_len = u16::try_from(self.input.len()).unwrap_or(u16::MAX);
        f.set_cursor_position((chunks[2].x + input_len + 1, chunks[2].y + 1));
    }
}
