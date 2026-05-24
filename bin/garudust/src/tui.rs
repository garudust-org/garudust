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
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Paragraph, Wrap},
    Terminal,
};
use tokio::sync::mpsc;

const SIDEBAR_W: u16 = 24;

fn new_session_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let h = nanos
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    format!("{:08x}", h as u32)
}

#[derive(Debug, Clone)]
pub enum AgentEvent {
    OutputChunk(String),
    Thinking,
    ToolCall(String),
    Done {
        iterations: u32,
        input_tokens: u32,
        output_tokens: u32,
    },
    Error(String),
    /// Sent after each agent turn and after model switch to refresh the sidebar.
    SidebarUpdate {
        toolsets: BTreeMap<String, Vec<String>>,
        skill_names: Vec<String>,
    },
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
    session_id: String,
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
            status: "Ready  ·  Enter to send  ·  Ctrl+C to quit".into(),
            scroll: 0,
            streaming: false,
            thinking_since: None,
            toolsets,
            skill_names,
            model,
            session_input_tokens: 0,
            session_output_tokens: 0,
            session_turns: 0,
            session_id: new_session_id(),
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
                                            tui.session_id = new_session_id();
                                            tui.session_input_tokens = 0;
                                            tui.session_output_tokens = 0;
                                            tui.session_turns = 0;
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
            AgentEvent::ToolCall(name) => {
                self.status = format!("▸ {name}");
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
                    .map(|c| format!(" · ~${c:.4}"))
                    .unwrap_or_default();
                let session_cost = estimate_cost_usd(
                    &self.model,
                    self.session_input_tokens,
                    self.session_output_tokens,
                )
                .map(|c| format!(" · session ~${c:.4}"))
                .unwrap_or_default();
                self.status = format!("{iterations} iter · {total} tok{cost_part}{session_cost}");
            }
            AgentEvent::Error(e) => {
                self.streaming = false;
                self.thinking_since = None;
                self.messages.push((Role::Error, format!("Error: {e}")));
                self.status = "Error — ready for next task".into();
                self.scroll = u16::MAX;
            }
            AgentEvent::SidebarUpdate {
                toolsets,
                skill_names,
            } => {
                self.toolsets = toolsets;
                self.skill_names = skill_names;
            }
        }
    }

    // Logo-only banner at the top of the chat pane.
    fn build_banner_lines(&self, pane_w: u16) -> Vec<Line<'static>> {
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
        let sep_style = Style::default().fg(Color::Rgb(45, 45, 45));

        let mut lines: Vec<Line<'static>> = vec![Line::from("")];
        for logo_str in LOGO {
            lines.push(Line::from(Span::styled(logo_str.to_string(), accent)));
        }
        lines.push(Line::from(Span::styled(
            "─".repeat(pane_w as usize),
            sep_style,
        )));
        lines
    }

    fn render_sidebar(&self, f: &mut ratatui::Frame, area: Rect) {
        let w = area.width as usize;

        let label_style = Style::default()
            .fg(Color::Rgb(170, 170, 170))
            .add_modifier(Modifier::BOLD);
        let toolset_style = Style::default().fg(Color::Rgb(90, 90, 90));
        let item_style = Style::default().fg(Color::Rgb(120, 120, 120));

        let mut lines: Vec<Line<'static>> = Vec::new();

        // Session ID
        lines.push(Line::from(Span::styled("SESSION", label_style)));
        lines.push(Line::from(Span::styled(self.session_id.clone(), item_style)));
        lines.push(Line::from(""));

        // Tools
        let tool_total: usize = self.toolsets.values().map(Vec::len).sum();
        lines.push(Line::from(Span::styled(
            format!("TOOLS ({tool_total})"),
            label_style,
        )));
        for (toolset, names) in &self.toolsets {
            let ts = format!(" {toolset}");
            let ts_display = if ts.len() > w { ts[..w].to_string() } else { ts };
            lines.push(Line::from(Span::styled(ts_display, toolset_style)));
            for name in names {
                let s = format!("  · {name}");
                let display = if s.len() > w { s[..w].to_string() } else { s };
                lines.push(Line::from(Span::styled(display, item_style)));
            }
        }
        lines.push(Line::from(""));

        // Skills
        let skill_total = self.skill_names.len();
        lines.push(Line::from(Span::styled(
            format!("SKILLS ({skill_total})"),
            label_style,
        )));
        if self.skill_names.is_empty() {
            lines.push(Line::from(Span::styled(" —", toolset_style)));
        } else {
            for name in &self.skill_names {
                let s = format!(" {name}");
                let display = if s.len() > w { s[..w].to_string() } else { s };
                lines.push(Line::from(Span::styled(display, item_style)));
            }
        }

        f.render_widget(Paragraph::new(Text::from(lines)), area);
    }

    fn render(&mut self, f: &mut ratatui::Frame) {
        let area = f.area();
        let w = area.width as usize;

        // 6 rows: header | sep | content | status | sep | input
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(3),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(area);

        // Content area: chat | vbar | sidebar
        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(20),
                Constraint::Length(1),
                Constraint::Length(SIDEBAR_W),
            ])
            .split(chunks[2]);

        let chat_area = content_chunks[0];
        let vbar_area = content_chunks[1];
        let sidebar_area = content_chunks[2];

        let accent = Style::default()
            .fg(Color::Rgb(245, 166, 35))
            .add_modifier(Modifier::BOLD);
        let dim = Style::default().fg(Color::Rgb(80, 80, 80));
        let sep_style = Style::default().fg(Color::Rgb(45, 45, 45));
        let text_style = Style::default().fg(Color::Rgb(210, 210, 210));
        let sep = "─".repeat(w);

        // ── Header ────────────────────────────────────────────────────────────
        let v = env!("CARGO_PKG_VERSION");
        let left = format!(" Garudust v{v}");
        let turns = self.session_turns;
        let cost_sfx = estimate_cost_usd(
            &self.model,
            self.session_input_tokens,
            self.session_output_tokens,
        )
        .map(|c| format!(" · ~${c:.4}"))
        .unwrap_or_default();
        let right = format!(
            "{} · {} turn{}{} ",
            self.model,
            turns,
            if turns == 1 { "" } else { "s" },
            cost_sfx,
        );
        let pad = w.saturating_sub(left.len() + right.len());

        let header = Line::from(vec![
            Span::styled(left, accent),
            Span::raw(" ".repeat(pad)),
            Span::styled(right, dim),
        ]);
        f.render_widget(Paragraph::new(header), chunks[0]);

        // ── Separator ─────────────────────────────────────────────────────────
        f.render_widget(Paragraph::new(sep.as_str()).style(sep_style), chunks[1]);

        // ── Vertical bar (chat | sidebar) ─────────────────────────────────────
        let vbar_lines: Vec<Line<'static>> = (0..vbar_area.height)
            .map(|_| Line::from(Span::styled("│", sep_style)))
            .collect();
        f.render_widget(Paragraph::new(Text::from(vbar_lines)), vbar_area);

        // ── Chat pane ─────────────────────────────────────────────────────────
        let banner = self.build_banner_lines(chat_area.width);

        let user_style = Style::default()
            .fg(Color::Rgb(99, 179, 237))
            .add_modifier(Modifier::BOLD);
        let ai_style = Style::default().fg(Color::Rgb(104, 211, 145));
        let err_style = Style::default().fg(Color::Rgb(252, 129, 129));

        let chat_lines: Vec<Line<'static>> = self
            .messages
            .iter()
            .flat_map(|(role, text)| -> Vec<Line<'static>> {
                let (prefix, prefix_style) = match role {
                    Role::User => ("  you › ", user_style),
                    Role::Assistant => ("   ai › ", ai_style),
                    Role::Error => ("  err › ", err_style),
                };
                text.lines()
                    .enumerate()
                    .map(move |(i, line)| {
                        if i == 0 {
                            Line::from(vec![
                                Span::styled(prefix.to_string(), prefix_style),
                                Span::styled(line.to_string(), text_style),
                            ])
                        } else {
                            Line::from(vec![
                                Span::raw("        "),
                                Span::styled(line.to_string(), text_style),
                            ])
                        }
                    })
                    .collect()
            })
            .collect();

        let all_lines: Vec<Line<'static>> = banner.into_iter().chain(chat_lines).collect();
        let visible = chat_area.height;

        let messages =
            Paragraph::new(Text::from(all_lines)).wrap(Wrap { trim: false });

        let total_visual =
            u16::try_from(messages.line_count(chat_area.width)).unwrap_or(u16::MAX);
        let max_scroll = total_visual.saturating_sub(visible);
        let scroll = if self.scroll == u16::MAX {
            max_scroll
        } else {
            self.scroll.min(max_scroll)
        };

        f.render_widget(messages.scroll((scroll, 0)), chat_area);

        // ── Sidebar ───────────────────────────────────────────────────────────
        self.render_sidebar(f, sidebar_area);

        // ── Status bar ────────────────────────────────────────────────────────
        let status_text = if let Some(since) = self.thinking_since {
            let secs = since.elapsed().as_secs();
            if secs > 0 {
                format!("  {} ({}s)", self.status, secs)
            } else {
                format!("  {}", self.status)
            }
        } else {
            format!("  {}", self.status)
        };
        f.render_widget(Paragraph::new(status_text.as_str()).style(dim), chunks[3]);

        // ── Separator ─────────────────────────────────────────────────────────
        f.render_widget(Paragraph::new(sep.as_str()).style(sep_style), chunks[4]);

        // ── Input line ────────────────────────────────────────────────────────
        let before = &self.input[..self.cursor];
        let at = self.input[self.cursor..]
            .chars()
            .next()
            .map_or_else(|| " ".to_string(), |c| c.to_string());
        let after_start = self.cursor
            + at.len().saturating_sub(if self.cursor < self.input.len() {
                at.len()
            } else {
                0
            });
        let after = if self.cursor < self.input.len() {
            &self.input[after_start..]
        } else {
            ""
        };

        let cursor_style = Style::default()
            .fg(Color::Black)
            .bg(Color::White)
            .add_modifier(Modifier::BOLD);

        let input_line = Line::from(vec![
            Span::styled(" ❯ ", accent),
            Span::styled(before.to_string(), text_style),
            Span::styled(at, cursor_style),
            Span::styled(after.to_string(), text_style),
        ]);

        f.render_widget(Paragraph::new(input_line), chunks[5]);
    }
}
