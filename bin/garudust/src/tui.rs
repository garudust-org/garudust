use std::collections::BTreeMap;
use std::io;

use garudust_core::pricing::estimate_cost_usd;

use crossterm::{
    cursor::{Hide, Show},
    event::{self, Event, KeyCode, KeyModifiers},
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
    /// Byte offset of the cursor in `input`.
    cursor: usize,
    messages: Vec<(Role, String)>,
    status: String,
    scroll: u16,
    streaming: bool,
    thinking_since: Option<std::time::Instant>,
    toolsets: BTreeMap<String, Vec<String>>,
    skill_names: Vec<String>,
    model: String,
    session_input_tokens: u32,
    session_output_tokens: u32,
    session_turns: u32,
}

#[derive(Clone)]
enum Role {
    User,
    Assistant,
    Error,
}

// ── Cursor helpers ────────────────────────────────────────────────────────────

/// Move byte offset one Unicode scalar value to the left.
fn prev_char_boundary(s: &str, cursor: usize) -> usize {
    let mut c = cursor;
    if c == 0 {
        return 0;
    }
    c -= 1;
    while c > 0 && !s.is_char_boundary(c) {
        c -= 1;
    }
    c
}

/// Move byte offset one Unicode scalar value to the right.
fn next_char_boundary(s: &str, cursor: usize) -> usize {
    if cursor >= s.len() {
        return s.len();
    }
    let mut c = cursor + 1;
    while c < s.len() && !s.is_char_boundary(c) {
        c += 1;
    }
    c
}

/// Jump left past a word boundary (Ctrl+Left).
fn prev_word(s: &str, cursor: usize) -> usize {
    let chars: Vec<(usize, char)> = s[..cursor].char_indices().collect();
    let mut i = chars.len();
    // skip trailing spaces
    while i > 0 && chars[i - 1].1 == ' ' {
        i -= 1;
    }
    // skip word chars
    while i > 0 && chars[i - 1].1 != ' ' {
        i -= 1;
    }
    chars.get(i).map_or(0, |(b, _)| *b)
}

/// Jump right past a word boundary (Ctrl+Right).
fn next_word(s: &str, cursor: usize) -> usize {
    let chars: Vec<(usize, char)> = s[cursor..].char_indices().collect();
    let mut i = 0;
    // skip trailing spaces
    while i < chars.len() && chars[i].1 == ' ' {
        i += 1;
    }
    // skip word chars
    while i < chars.len() && chars[i].1 != ' ' {
        i += 1;
    }
    chars.get(i).map_or(s.len(), |(b, _)| cursor + *b)
}

impl Tui {
    pub fn new(
        toolsets: BTreeMap<String, Vec<String>>,
        skill_names: Vec<String>,
        model: String,
    ) -> Self {
        Self {
            input: String::new(),
            cursor: 0,
            messages: Vec::new(),
            status: "Ready — press Enter to send, Ctrl+C to quit".into(),
            scroll: 0,
            streaming: false,
            thinking_since: None,
            toolsets,
            skill_names,
            model,
            session_input_tokens: 0,
            session_output_tokens: 0,
            session_turns: 0,
        }
    }

    // ── Input editing ─────────────────────────────────────────────────────────

    fn insert_char(&mut self, c: char) {
        self.input.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    fn delete_before_cursor(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let prev = prev_char_boundary(&self.input, self.cursor);
        self.input.drain(prev..self.cursor);
        self.cursor = prev;
    }

    fn delete_after_cursor(&mut self) {
        if self.cursor >= self.input.len() {
            return;
        }
        let next = next_char_boundary(&self.input, self.cursor);
        self.input.drain(self.cursor..next);
    }

    fn move_left(&mut self) {
        self.cursor = prev_char_boundary(&self.input, self.cursor);
    }

    fn move_right(&mut self) {
        self.cursor = next_char_boundary(&self.input, self.cursor);
    }

    fn move_word_left(&mut self) {
        self.cursor = prev_word(&self.input, self.cursor);
    }

    fn move_word_right(&mut self) {
        self.cursor = next_word(&self.input, self.cursor);
    }

    fn move_home(&mut self) {
        self.cursor = 0;
    }

    fn move_end(&mut self) {
        self.cursor = self.input.len();
    }

    fn clear_input(&mut self) {
        self.input.clear();
        self.cursor = 0;
    }

    // ── Main loop ─────────────────────────────────────────────────────────────

    pub async fn run(
        tx_event: mpsc::Sender<TuiEvent>,
        mut rx_agent: mpsc::Receiver<AgentEvent>,
        toolsets: BTreeMap<String, Vec<String>>,
        skill_names: Vec<String>,
        model: String,
    ) -> io::Result<()> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, Hide)?;
        let backend = CrosstermBackend::new(stdout);
        let mut term = Terminal::new(backend)?;

        let mut tui = Tui::new(toolsets, skill_names, model);
        tui.messages.push((
            Role::Assistant,
            "Type your task and press Enter.  /help for commands.".into(),
        ));

        loop {
            while let Ok(ev) = rx_agent.try_recv() {
                tui.handle_agent_event(ev);
            }

            term.draw(|f| tui.render(f))?;

            if event::poll(std::time::Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    match (key.code, key.modifiers) {
                        (KeyCode::Char('c' | 'q'), KeyModifiers::CONTROL) => {
                            let _ = tx_event.send(TuiEvent::Quit).await;
                            break;
                        }
                        (KeyCode::Enter, _) => {
                            let text = tui.input.trim().to_string();
                            tui.clear_input();
                            if !text.is_empty() {
                                if let Some(rest) = text.strip_prefix('/') {
                                    let (cmd, args) = rest
                                        .split_once(' ')
                                        .map_or((rest, None), |(c, a)| (c, Some(a.trim())));
                                    match cmd {
                                        "new" | "clear" => {
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
                                                "/new|/clear — start fresh (clears history)\n\
                                                 /model <n>  — switch to a different model\n\
                                                 /help       — show this help"
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

                        // ── Cursor movement ───────────────────────────────────
                        (KeyCode::Left, KeyModifiers::CONTROL) => tui.move_word_left(),
                        (KeyCode::Right, KeyModifiers::CONTROL) => tui.move_word_right(),
                        (KeyCode::Left, _) => tui.move_left(),
                        (KeyCode::Right, _) => tui.move_right(),
                        (KeyCode::Home, _) | (KeyCode::Char('a'), KeyModifiers::CONTROL) => {
                            tui.move_home();
                        }
                        (KeyCode::End, _) | (KeyCode::Char('e'), KeyModifiers::CONTROL) => {
                            tui.move_end();
                        }

                        // ── Deletion ──────────────────────────────────────────
                        (KeyCode::Backspace, _) => tui.delete_before_cursor(),
                        (KeyCode::Delete, _) => tui.delete_after_cursor(),
                        // Ctrl+U — kill line (clear all input)
                        (KeyCode::Char('u'), KeyModifiers::CONTROL) => tui.clear_input(),

                        // ── Scroll ────────────────────────────────────────────
                        (KeyCode::Up | KeyCode::PageUp, _) => {
                            tui.scroll = tui.scroll.saturating_sub(1);
                        }
                        (KeyCode::Down | KeyCode::PageDown, _) => {
                            tui.scroll = tui.scroll.saturating_add(1);
                        }

                        // ── Character input ───────────────────────────────────
                        (KeyCode::Char(c), _) => tui.insert_char(c),

                        _ => {}
                    }
                }
            }
        }

        disable_raw_mode()?;
        execute!(term.backend_mut(), LeaveAlternateScreen, Show)?;
        Ok(())
    }

    fn handle_agent_event(&mut self, ev: AgentEvent) {
        match ev {
            AgentEvent::OutputChunk(delta) => {
                if self.streaming {
                    if let Some((Role::Assistant, buf)) = self.messages.last_mut() {
                        buf.push_str(&delta);
                    }
                } else {
                    self.streaming = true;
                    self.status = "Streaming…".into();
                    self.messages.push((Role::Assistant, delta));
                }
                self.scroll = u16::MAX;
            }
            AgentEvent::Thinking => {
                self.streaming = false;
                self.thinking_since = Some(std::time::Instant::now());
                self.status = "Thinking…".into();
            }
            AgentEvent::Done {
                iterations,
                input_tokens,
                output_tokens,
            } => {
                self.streaming = false;
                self.thinking_since = None;
                self.session_turns += 1;
                self.session_input_tokens += input_tokens;
                self.session_output_tokens += output_tokens;
                let total = input_tokens + output_tokens;
                let cost_part = estimate_cost_usd(&self.model, input_tokens, output_tokens)
                    .map(|c| format!(" | ~${c:.4}"))
                    .unwrap_or_default();
                let session_cost = estimate_cost_usd(
                    &self.model,
                    self.session_input_tokens,
                    self.session_output_tokens,
                )
                .map(|c| format!(" | session ~${c:.4}"))
                .unwrap_or_default();
                self.status =
                    format!("Done — {iterations} iter | {total} tok{cost_part}{session_cost}");
            }
            AgentEvent::Error(e) => {
                self.streaming = false;
                self.thinking_since = None;
                self.messages.push((Role::Error, format!("Error: {e}")));
                self.status = "Error — ready for next task".into();
                self.scroll = u16::MAX;
            }
        }
    }

    // Build the startup banner lines (logo left, tools+skills right).
    // `pane_w` is the full width of the messages block including borders.
    fn build_banner_lines(&self, pane_w: u16) -> Vec<Line<'static>> {
        const LOGO_W: usize = 20;
        const LOGO: &[&str] = &[
            "        ★          ",
            "       /|\\       ",
            "   \\\\ (◉ ◉) //  ",
            "    \\\\  ▼  //   ",
            "     \\\\ | //    ",
            "      \\|||/      ",
            "       |||       ",
            "      /   \\     ",
            "                  ",
        ];

        let accent = Style::default()
            .fg(Color::Rgb(245, 166, 35))
            .add_modifier(Modifier::BOLD);
        let dim = Style::default().fg(Color::DarkGray);
        let bold_w = Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD);
        let normal = Style::default().fg(Color::White);

        let inner_w = pane_w.saturating_sub(2) as usize;
        let right_w = inner_w.saturating_sub(LOGO_W);

        let mut right: Vec<Vec<Span<'static>>> = Vec::new();

        let tool_total: usize = self.toolsets.values().map(Vec::len).sum();
        right.push(vec![Span::styled(
            format!("Available Tools ({tool_total})"),
            bold_w,
        )]);

        for (toolset, names) in &self.toolsets {
            let prefix = format!("  {toolset}: ");
            let avail = right_w.saturating_sub(prefix.len());
            let joined = names.join(", ");
            let display = if joined.len() > avail && avail > 3 {
                format!("{}...", &joined[..avail.saturating_sub(3)])
            } else {
                joined
            };
            right.push(vec![
                Span::styled(prefix, dim),
                Span::styled(display, normal),
            ]);
        }

        right.push(vec![Span::raw("")]);

        let skill_total = self.skill_names.len();
        right.push(vec![Span::styled(
            format!("Available Skills ({skill_total})"),
            bold_w,
        )]);

        if self.skill_names.is_empty() {
            right.push(vec![Span::styled("  —", dim)]);
        } else {
            let joined = self.skill_names.join(", ");
            let avail = right_w.saturating_sub(2);
            let display = if joined.len() > avail && avail > 3 {
                format!("{}...", &joined[..avail.saturating_sub(3)])
            } else {
                joined
            };
            right.push(vec![Span::styled(format!("  {display}"), normal)]);
        }

        let n_rows = LOGO.len().max(right.len());
        let mut lines: Vec<Line<'static>> = Vec::new();

        let v = env!("CARGO_PKG_VERSION");
        lines.push(Line::from(vec![
            Span::styled(format!(" Garudust v{v}"), accent),
            Span::styled(
                format!(" · {tool_total} tools · {skill_total} skills · /help for commands"),
                dim,
            ),
        ]));

        for i in 0..n_rows {
            let logo_str = LOGO.get(i).copied().unwrap_or("");
            let pad = LOGO_W.saturating_sub(logo_str.chars().count());
            let logo_padded = format!("{logo_str}{:>pad$}", "", pad = pad);

            let mut spans: Vec<Span<'static>> = vec![Span::styled(logo_padded, accent)];
            if let Some(right_spans) = right.get(i).cloned() {
                spans.extend(right_spans);
            }
            lines.push(Line::from(spans));
        }

        lines.push(Line::from(Span::styled(
            "─".repeat(inner_w),
            Style::default().fg(Color::Rgb(60, 60, 60)),
        )));

        lines
    }

    fn render(&mut self, f: &mut ratatui::Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),
                Constraint::Length(1),
                Constraint::Length(3),
            ])
            .split(f.area());

        // ── Chat pane ──
        let banner = self.build_banner_lines(chunks[0].width);

        let chat_lines: Vec<Line<'static>> = self
            .messages
            .iter()
            .flat_map(|(role, text)| -> Vec<Line<'static>> {
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
                                Span::styled(prefix.to_string(), style),
                                Span::raw(line.to_string()),
                            ])
                        } else {
                            Line::from(vec![Span::raw("       "), Span::raw(line.to_string())])
                        }
                    })
                    .collect()
            })
            .collect();

        let all_lines: Vec<Line<'static>> = banner.into_iter().chain(chat_lines).collect();
        let visible = chunks[0].height.saturating_sub(2);

        let messages = Paragraph::new(Text::from(all_lines))
            .block(Block::default().borders(Borders::ALL).title(" Garudust "))
            .wrap(Wrap { trim: false });

        let text_w = chunks[0].width.saturating_sub(2);
        let total_visual = u16::try_from(messages.line_count(text_w)).unwrap_or(u16::MAX);
        let max_scroll = total_visual.saturating_sub(visible + 2);
        let scroll = if self.scroll == u16::MAX {
            max_scroll
        } else {
            self.scroll.min(max_scroll)
        };

        f.render_widget(messages.scroll((scroll, 0)), chunks[0]);

        // ── Status bar ──
        let status_text = if let Some(since) = self.thinking_since {
            let secs = since.elapsed().as_secs();
            if secs > 0 {
                format!("{} ({}s)", self.status, secs)
            } else {
                self.status.clone()
            }
        } else {
            self.status.clone()
        };
        f.render_widget(
            Paragraph::new(status_text.as_str()).style(Style::default().fg(Color::DarkGray)),
            chunks[1],
        );

        // ── Input box with cursor ──
        // Render text before and after cursor with a block cursor style.
        let before = &self.input[..self.cursor];
        let at = self.input[self.cursor..]
            .chars()
            .next()
            .map_or_else(|| " ".to_string(), |c| c.to_string());
        let after_start = self.cursor
            + at.len().saturating_sub(
                // at is always 1 char from the string; if we appended a space, skip 0 bytes
                if self.cursor < self.input.len() {
                    at.len()
                } else {
                    0
                },
            );
        let after = if self.cursor < self.input.len() {
            &self.input[after_start..]
        } else {
            ""
        };

        let cursor_style = Style::default()
            .fg(Color::Black)
            .bg(Color::White)
            .add_modifier(Modifier::BOLD);

        let input_spans = Line::from(vec![
            Span::raw(before.to_string()),
            Span::styled(at, cursor_style),
            Span::raw(after.to_string()),
        ]);

        let input_widget = Paragraph::new(input_spans)
            .block(Block::default().borders(Borders::ALL).title(" Input "))
            .style(Style::default().fg(Color::White));
        f.render_widget(input_widget, chunks[2]);
    }
}
