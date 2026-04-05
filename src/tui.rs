use std::io;
use std::path::Path;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, Tabs, Wrap},
};

use crate::insights::{build_insights, diagnosis_for};
use crate::report::{AnalysisReport, CATEGORIES};

const TAB_TITLES: [&str; 3] = ["Overview", "Syscalls", "Raw trace"];

#[derive(Debug, Default)]
struct TuiState {
    active_tab: usize,
    raw_scroll: u16,
}

pub fn run_tui(
    command_text: &str,
    exit_code: i32,
    wall_time: Duration,
    kept_trace_path: Option<&Path>,
    timed_out: bool,
    timeout_secs: u64,
    report: &AnalysisReport,
    raw_trace: &str,
    no_color: bool,
) -> io::Result<()> {
    let mut terminal = ratatui::init();
    let result = run_app(
        &mut terminal,
        command_text,
        exit_code,
        wall_time,
        kept_trace_path,
        timed_out,
        timeout_secs,
        report,
        raw_trace,
        no_color,
    );
    ratatui::restore();
    result
}

fn run_app(
    terminal: &mut ratatui::DefaultTerminal,
    command_text: &str,
    exit_code: i32,
    wall_time: Duration,
    kept_trace_path: Option<&Path>,
    timed_out: bool,
    timeout_secs: u64,
    report: &AnalysisReport,
    raw_trace: &str,
    no_color: bool,
) -> io::Result<()> {
    let mut state = TuiState::default();

    loop {
        terminal.draw(|frame| {
            draw_ui(
                frame,
                &state,
                command_text,
                exit_code,
                wall_time,
                kept_trace_path,
                timed_out,
                timeout_secs,
                report,
                raw_trace,
                no_color,
            )
        })?;

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Release {
                    continue;
                }

                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc | KeyCode::Enter => return Ok(()),

                    KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => {
                        state.active_tab = (state.active_tab + 1) % TAB_TITLES.len();
                    }

                    KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') => {
                        state.active_tab = if state.active_tab == 0 {
                            TAB_TITLES.len() - 1
                        } else {
                            state.active_tab - 1
                        };
                    }

                    KeyCode::Down | KeyCode::Char('j') => {
                        if state.active_tab == 2 {
                            state.raw_scroll = state.raw_scroll.saturating_add(1);
                        }
                    }

                    KeyCode::Up | KeyCode::Char('k') => {
                        if state.active_tab == 2 {
                            state.raw_scroll = state.raw_scroll.saturating_sub(1);
                        }
                    }

                    KeyCode::Home => {
                        if state.active_tab == 2 {
                            state.raw_scroll = 0;
                        }
                    }

                    KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        if state.active_tab == 2 {
                            state.raw_scroll = state.raw_scroll.saturating_sub(10);
                        }
                    }

                    KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        if state.active_tab == 2 {
                            state.raw_scroll = state.raw_scroll.saturating_add(10);
                        }
                    }

                    _ => {}
                }
            }
        }
    }
}

fn draw_ui(
    frame: &mut Frame,
    state: &TuiState,
    command_text: &str,
    exit_code: i32,
    wall_time: Duration,
    kept_trace_path: Option<&Path>,
    timed_out: bool,
    timeout_secs: u64,
    report: &AnalysisReport,
    raw_trace: &str,
    no_color: bool,
) {
    let area = frame.area();

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(4),
            Constraint::Length(3),
            Constraint::Min(12),
            Constraint::Length(3),
        ])
        .split(area);

    let header = Paragraph::new(header_text(
        command_text,
        exit_code,
        wall_time,
        timed_out,
        timeout_secs,
        report.top_category_by_time,
        no_color,
    ))
    .block(panel("why tui", no_color))
    .wrap(Wrap { trim: true });

    frame.render_widget(header, vertical[0]);

    let tabs = Tabs::new(TAB_TITLES)
        .select(state.active_tab)
        .block(panel("Views", no_color))
        .style(Style::default().fg(muted(no_color)))
        .highlight_style(
            Style::default()
                .fg(accent(no_color))
                .add_modifier(Modifier::BOLD),
        )
        .divider(" │ ");

    frame.render_widget(tabs, vertical[1]);

    match state.active_tab {
        0 => render_overview(
            frame,
            vertical[2],
            exit_code,
            wall_time,
            kept_trace_path,
            timed_out,
            timeout_secs,
            report,
            no_color,
        ),
        1 => render_syscalls(frame, vertical[2], report, no_color),
        2 => render_raw_trace(frame, vertical[2], raw_trace, state.raw_scroll, no_color),
        _ => {}
    }

    let footer = Paragraph::new(footer_text(state.active_tab, no_color))
        .block(panel("Controls", no_color))
        .wrap(Wrap { trim: true });

    frame.render_widget(footer, vertical[3]);
}

fn render_overview(
    frame: &mut Frame,
    area: Rect,
    exit_code: i32,
    wall_time: Duration,
    kept_trace_path: Option<&Path>,
    timed_out: bool,
    timeout_secs: u64,
    report: &AnalysisReport,
    no_color: bool,
) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let left = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(7), Constraint::Min(8)])
        .split(cols[0]);

    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(7), Constraint::Min(8)])
        .split(cols[1]);

    let summary = Paragraph::new(summary_text(report, no_color))
        .block(panel("Summary", no_color))
        .wrap(Wrap { trim: true });
    frame.render_widget(summary, left[0]);

    let insights = Paragraph::new(insights_text(report, no_color))
        .block(panel("Insights", no_color))
        .wrap(Wrap { trim: true });
    frame.render_widget(insights, left[1]);

    let takeaway = Paragraph::new(takeaway_text(
        exit_code,
        wall_time,
        kept_trace_path,
        timed_out,
        timeout_secs,
        report,
        no_color,
    ))
    .block(panel("Takeaway", no_color))
    .wrap(Wrap { trim: true });
    frame.render_widget(takeaway, right[0]);

    let syscalls = Paragraph::new(top_syscalls_text(report, no_color))
        .block(panel("Top syscalls", no_color))
        .wrap(Wrap { trim: true });
    frame.render_widget(syscalls, right[1]);
}

fn render_syscalls(frame: &mut Frame, area: Rect, report: &AnalysisReport, no_color: bool) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
        .split(area);

    let left = Paragraph::new(top_syscalls_text(report, no_color))
        .block(panel("Top syscalls", no_color))
        .wrap(Wrap { trim: true });

    frame.render_widget(left, cols[0]);

    let right = Paragraph::new(syscalls_side_text(report, no_color))
        .block(panel("Quick stats", no_color))
        .wrap(Wrap { trim: true });

    frame.render_widget(right, cols[1]);
}

fn render_raw_trace(
    frame: &mut Frame,
    area: Rect,
    raw_trace: &str,
    raw_scroll: u16,
    no_color: bool,
) {
    let total_lines = raw_trace.lines().count();

    let title = format!(" Raw trace  scroll:{}  lines:{} ", raw_scroll, total_lines);

    let widget = Paragraph::new(raw_trace)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .title_style(
                    Style::default()
                        .fg(accent(no_color))
                        .add_modifier(Modifier::BOLD),
                )
                .border_style(Style::default().fg(muted(no_color))),
        )
        .wrap(Wrap { trim: false })
        .scroll((raw_scroll, 0));

    frame.render_widget(widget, area);
}

fn header_text(
    command_text: &str,
    exit_code: i32,
    wall_time: Duration,
    timed_out: bool,
    timeout_secs: u64,
    top_category: &str,
    no_color: bool,
) -> Text<'static> {
    Text::from(vec![
        Line::from(vec![
            Span::styled(
                "Command: ",
                Style::default()
                    .fg(accent(no_color))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(command_text.to_string()),
        ]),
        Line::from(vec![
            Span::styled(
                "Status: ",
                Style::default()
                    .fg(accent(no_color))
                    .add_modifier(Modifier::BOLD),
            ),
            status_span(exit_code, timed_out, timeout_secs, no_color),
            Span::raw("   "),
            Span::styled(
                "Dominant: ",
                Style::default()
                    .fg(accent(no_color))
                    .add_modifier(Modifier::BOLD),
            ),
            dominant_span(top_category, no_color),
            Span::raw("   "),
            Span::styled(
                "Wall time: ",
                Style::default()
                    .fg(accent(no_color))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!("{:.2?}", wall_time)),
        ]),
    ])
}

fn takeaway_text(
    exit_code: i32,
    wall_time: Duration,
    kept_trace_path: Option<&Path>,
    timed_out: bool,
    timeout_secs: u64,
    report: &AnalysisReport,
    no_color: bool,
) -> Text<'static> {
    Text::from(vec![
        Line::from(vec![Span::styled(
            short_diagnosis(report.top_category_by_time),
            Style::default()
                .fg(success(no_color))
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            short_hint(report.top_category_by_time),
            Style::default().fg(warning(no_color)),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "Status: ",
                Style::default()
                    .fg(accent(no_color))
                    .add_modifier(Modifier::BOLD),
            ),
            status_span(exit_code, timed_out, timeout_secs, no_color),
        ]),
        Line::from(vec![
            Span::styled(
                "Wall time: ",
                Style::default()
                    .fg(accent(no_color))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!("{:.2?}", wall_time)),
        ]),
        Line::from(vec![
            Span::styled(
                "Trace: ",
                Style::default()
                    .fg(accent(no_color))
                    .add_modifier(Modifier::BOLD),
            ),
            match kept_trace_path {
                Some(path) => Span::styled(
                    path.display().to_string(),
                    Style::default().fg(muted(no_color)),
                ),
                None => Span::styled(
                    "temporary (auto-deleted)".to_string(),
                    Style::default().fg(muted(no_color)),
                ),
            },
        ]),
    ])
}

fn summary_text(report: &AnalysisReport, no_color: bool) -> Text<'static> {
    let mut lines = Vec::new();

    for category in CATEGORIES {
        let count = report.counts.get(category).unwrap_or(&0);
        let time_spent = report.times.get(category).unwrap_or(&0.0);

        let percent = if report.total_traced_time > 0.0 {
            (*time_spent / report.total_traced_time) * 100.0
        } else {
            0.0
        };

        let line = format!(
            "{:<8} {:>4} calls   {:>9.6}s   {:>5.1}%",
            capitalize(category),
            count,
            time_spent,
            percent
        );

        let style = if category == report.top_category_by_time {
            Style::default()
                .fg(category_color(category, no_color))
                .add_modifier(Modifier::BOLD)
        } else if no_color {
            Style::default()
        } else {
            Style::default().add_modifier(Modifier::DIM)
        };

        lines.push(Line::from(vec![Span::styled(line, style)]));
    }

    Text::from(lines)
}

fn insights_text(report: &AnalysisReport, no_color: bool) -> Text<'static> {
    let file_calls = *report.counts.get("file").unwrap_or(&0);
    let network_calls = *report.counts.get("network").unwrap_or(&0);
    let process_calls = *report.counts.get("process").unwrap_or(&0);
    let wait_calls = *report.counts.get("wait").unwrap_or(&0);

    let file_time = *report.times.get("file").unwrap_or(&0.0);
    let network_time = *report.times.get("network").unwrap_or(&0.0);
    let process_time = *report.times.get("process").unwrap_or(&0.0);
    let wait_time = *report.times.get("wait").unwrap_or(&0.0);

    let insights = build_insights(
        file_calls,
        network_calls,
        process_calls,
        wait_calls,
        file_time,
        network_time,
        process_time,
        wait_time,
        report.total_traced_time,
    );

    let mut lines = Vec::new();

    if insights.is_empty() {
        lines.push(Line::from("No extra insights yet."));
    } else {
        for item in insights {
            lines.push(Line::from(vec![
                Span::styled("• ", Style::default().fg(warning(no_color))),
                Span::raw(item),
            ]));
        }
    }

    Text::from(lines)
}

fn top_syscalls_text(report: &AnalysisReport, no_color: bool) -> Text<'static> {
    let mut lines = Vec::new();

    if report.top_syscalls.is_empty() {
        lines.push(Line::from("No syscall data yet."));
    } else {
        for (index, (name, count)) in report.top_syscalls.iter().enumerate() {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{}. ", index + 1),
                    Style::default().fg(muted(no_color)),
                ),
                Span::styled(
                    format!("{:<14}", name),
                    Style::default().fg(info(no_color)),
                ),
                Span::styled(
                    format!("{:>4}", count),
                    Style::default().fg(muted(no_color)),
                ),
            ]));
        }
    }

    Text::from(lines)
}

fn syscalls_side_text(report: &AnalysisReport, no_color: bool) -> Text<'static> {
    let mut lines = vec![
        Line::from(vec![
            Span::styled(
                "Dominant: ",
                Style::default()
                    .fg(accent(no_color))
                    .add_modifier(Modifier::BOLD),
            ),
            dominant_span(report.top_category_by_time, no_color),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            short_diagnosis(report.top_category_by_time),
            Style::default().fg(success(no_color)),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "Total traced: ",
                Style::default()
                    .fg(accent(no_color))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!("{:.6}s", report.total_traced_time)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Category totals",
            Style::default()
                .fg(accent(no_color))
                .add_modifier(Modifier::BOLD),
        )]),
    ];

    for category in CATEGORIES {
        let count = report.counts.get(category).unwrap_or(&0);
        let time_spent = report.times.get(category).unwrap_or(&0.0);
        lines.push(Line::from(format!(
            "{}: {} calls, {:.6}s",
            capitalize(category),
            count,
            time_spent
        )));
    }

    Text::from(lines)
}

fn footer_text(active_tab: usize, no_color: bool) -> Text<'static> {
    let raw_extra = if active_tab == 2 {
        "   j/k or ↑/↓ scroll   ctrl+d / ctrl+u jump"
    } else {
        ""
    };

    Text::from(vec![Line::from(vec![
        Span::styled(
            "tab",
            Style::default()
                .fg(accent(no_color))
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" next   "),
        Span::styled(
            "shift+tab",
            Style::default()
                .fg(accent(no_color))
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" prev   "),
        Span::styled(
            "←/→",
            Style::default()
                .fg(accent(no_color))
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" switch   "),
        Span::styled(
            "q",
            Style::default()
                .fg(accent(no_color))
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(" quit{}", raw_extra)),
    ])])
}

fn panel(title: &str, no_color: bool) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", title))
        .title_style(
            Style::default()
                .fg(accent(no_color))
                .add_modifier(Modifier::BOLD),
        )
        .border_style(Style::default().fg(muted(no_color)))
}

fn status_span(exit_code: i32, timed_out: bool, timeout_secs: u64, no_color: bool) -> Span<'static> {
    if timed_out {
        Span::styled(
            format!("timed out after {}s", timeout_secs),
            Style::default()
                .fg(danger(no_color))
                .add_modifier(Modifier::BOLD),
        )
    } else if exit_code == 0 {
        Span::styled("ok".to_string(), Style::default().fg(success(no_color)))
    } else {
        Span::styled(
            format!("exit code {}", exit_code),
            Style::default()
                .fg(danger(no_color))
                .add_modifier(Modifier::BOLD),
        )
    }
}

fn dominant_span(category: &str, no_color: bool) -> Span<'static> {
    Span::styled(
        capitalize(category),
        Style::default()
            .fg(category_color(category, no_color))
            .add_modifier(Modifier::BOLD),
    )
}

fn short_diagnosis(category: &str) -> &'static str {
    match category {
        "file" => "Mostly file work",
        "network" => "Mostly network work",
        "process" => "Mostly process work",
        "wait" => "Mostly waiting / sleeping",
        _ => diagnosis_for(category),
    }
}

fn short_hint(category: &str) -> &'static str {
    match category {
        "file" => "Usually file reads, writes, metadata checks, or directory scanning.",
        "network" => "Usually connecting, sending, receiving, or socket-heavy work.",
        "process" => "Usually spawning child processes or waiting for them.",
        "wait" => "Usually sleep, blocking I/O, socket wait, or lock wait.",
        _ => "Try a command with more visible activity.",
    }
}

fn accent(no_color: bool) -> Color {
    if no_color { Color::Reset } else { Color::Cyan }
}

fn success(no_color: bool) -> Color {
    if no_color { Color::Reset } else { Color::Green }
}

fn warning(no_color: bool) -> Color {
    if no_color { Color::Reset } else { Color::Yellow }
}

fn danger(no_color: bool) -> Color {
    if no_color { Color::Reset } else { Color::Red }
}

fn info(no_color: bool) -> Color {
    if no_color { Color::Reset } else { Color::Magenta }
}

fn muted(no_color: bool) -> Color {
    if no_color { Color::Reset } else { Color::DarkGray }
}

fn category_color(category: &str, no_color: bool) -> Color {
    if no_color {
        Color::Reset
    } else {
        match category {
            "wait" => Color::Red,
            "network" => Color::Magenta,
            "file" => Color::Blue,
            "process" => Color::Yellow,
            _ => Color::White,
        }
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}