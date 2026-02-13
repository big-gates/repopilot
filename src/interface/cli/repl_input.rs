//! REPL 입력 처리기.
//! `/`로 시작하면 입력 중 실시간으로 명령 추천을 표시한다.

use std::env;
use std::io::{self, IsTerminal, Write};

use anyhow::Result;
use crossterm::cursor;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::style::{Color, ResetColor, SetBackgroundColor, SetForegroundColor};
use crossterm::terminal::{self, ClearType};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::domain::target::ReviewTarget;

struct Suggestion {
    slash: &'static str,
    description: &'static str,
    usage: &'static str,
}

const SUGGESTIONS: [Suggestion; 3] = [
    Suggestion {
        slash: "/config",
        description: "show effective merged config",
        usage: "/config [edit]",
    },
    Suggestion {
        slash: "/review",
        description: "run review for PR/MR URL",
        usage: "/review <url> [--dry-run] [--force]",
    },
    Suggestion {
        slash: "/exit",
        description: "exit interactive shell",
        usage: "/exit",
    },
];
const DEFAULT_INPUT_PREFILL: &str = "";
// 입력 영역 기본 높이: 상단 구분선 + 입력줄 + 하단 구분선
const PANEL_BASE_HEIGHT: usize = 3;
const PANEL_BOTTOM_PADDING: usize = 0;

/// REPL 한 줄 입력을 읽는다.
/// - TTY + 지원 터미널: 실시간 추천 + 방향키 선택
/// - non-TTY/미지원 터미널: 일반 라인 입력
pub fn read_repl_input(prefill: Option<&str>) -> Result<Option<String>> {
    let initial = prefill.unwrap_or(DEFAULT_INPUT_PREFILL);

    if !supports_interactive_input() {
        return read_line_fallback(initial);
    }

    match read_line_interactive(initial) {
        Ok(v) => Ok(v),
        Err(_) => read_line_fallback(initial),
    }
}

fn supports_interactive_input() -> bool {
    if !io::stdout().is_terminal() {
        return false;
    }

    // dumb 터미널에서는 제어 시퀀스 기반 UI를 비활성화한다.
    if let Ok(term) = env::var("TERM") && term.eq_ignore_ascii_case("dumb") {
        return false;
    }

    true
}

fn read_line_fallback(initial: &str) -> Result<Option<String>> {
    // 대체 입력 모드에서도 프리필 문자열을 동일하게 적용한다.
    print!("prpilot> {initial}");
    io::stdout().flush()?;

    let mut line = String::new();
    let read = io::stdin().read_line(&mut line)?;
    if read == 0 {
        return Ok(None);
    }

    let typed = trim_newline(line);
    if initial.is_empty() || typed.starts_with('/') || typed.starts_with(initial) {
        return Ok(Some(typed));
    }

    Ok(Some(format!("{initial}{typed}")))
}

fn read_line_interactive(initial: &str) -> Result<Option<String>> {
    let mut stdout = io::stdout();
    let _guard = InputGuard::enter(&mut stdout)?;

    let mut input = initial.to_string();
    let mut cursor_chars = input.chars().count();
    let mut selected_idx = default_suggestion_index(&match_suggestions(&input));

    loop {
        let suggestions = match_suggestions(&input);
        if suggestions.is_empty() {
            selected_idx = 0;
        } else if selected_idx >= suggestions.len() {
            selected_idx = suggestions.len() - 1;
        }

        render_frame(&mut stdout, &input, cursor_chars, &suggestions, selected_idx)?;

        match event::read()? {
            Event::Paste(text) => {
                for ch in text.chars() {
                    insert_char_at(&mut input, cursor_chars, ch);
                    cursor_chars += 1;
                }
            }
            Event::Key(key) => {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                match key.code {
            KeyCode::Enter => {
                // 인자가 필요한 명령은 즉시 실행하지 않고 인자 입력 상태로 확장한다.
                if let Some(expanded) = expand_input(&input, &suggestions, selected_idx) {
                    input = expanded;
                    cursor_chars = input.chars().count();
                    continue;
                }
                let final_input = finalize_input(&input, &suggestions, selected_idx);
                clear_panel_for_output(&mut stdout)?;
                return Ok(Some(final_input));
            }
            KeyCode::Backspace => {
                if cursor_chars > 0 {
                    remove_char_at(&mut input, cursor_chars - 1);
                    cursor_chars -= 1;
                }
            }
            KeyCode::Delete => {
                if cursor_chars < input.chars().count() {
                    remove_char_at(&mut input, cursor_chars);
                }
            }
            KeyCode::Left => {
                cursor_chars = cursor_chars.saturating_sub(1);
            }
            KeyCode::Right => {
                cursor_chars = (cursor_chars + 1).min(input.chars().count());
            }
            KeyCode::Home => {
                cursor_chars = 0;
            }
            KeyCode::End => {
                cursor_chars = input.chars().count();
            }
            KeyCode::Up => {
                if !suggestions.is_empty() {
                    selected_idx = selected_idx.saturating_sub(1);
                }
            }
            KeyCode::Down => {
                if !suggestions.is_empty() {
                    selected_idx = (selected_idx + 1).min(suggestions.len() - 1);
                }
            }
            KeyCode::Tab => {
                if !suggestions.is_empty() && input.starts_with('/') && !input.contains(' ') {
                    input = suggestions[selected_idx].slash.to_string();
                    cursor_chars = input.chars().count();
                } else if let Some(completed) = complete_subcommand(&input) {
                    input = completed;
                    cursor_chars = input.chars().count();
                }
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                clear_panel_for_output(&mut stdout)?;
                return Ok(None);
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                clear_panel_for_output(&mut stdout)?;
                return Ok(Some("/exit".to_string()));
            }
            KeyCode::Char(ch) => {
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::ALT)
                {
                    insert_char_at(&mut input, cursor_chars, ch);
                    cursor_chars += 1;
                }
            }
            _ => {}
                }
            }
            _ => {}
        }
    }
}

fn match_suggestions(input: &str) -> Vec<&'static Suggestion> {
    if !input.starts_with('/') {
        return Vec::new();
    }

    if input.contains(' ') {
        return Vec::new();
    }

    let q = input.to_ascii_lowercase();
    SUGGESTIONS
        .iter()
        .filter(|s| s.slash.starts_with(&q) || q == "/")
        .collect()
}

fn default_suggestion_index(suggestions: &[&Suggestion]) -> usize {
    suggestions
        .iter()
        .position(|item| item.slash == "/review")
        .unwrap_or(0)
}

fn finalize_input(input: &str, suggestions: &[&Suggestion], selected_idx: usize) -> String {
    if input.starts_with('/') && !input.contains(' ') && !suggestions.is_empty() {
        return suggestions[selected_idx].slash.to_string();
    }
    input.to_string()
}

/// 인자가 필요한 명령을 단독 입력 시 공백을 추가해 확장한다.
fn expand_input(input: &str, suggestions: &[&Suggestion], selected_idx: usize) -> Option<String> {
    if input.contains(' ') {
        return None;
    }

    const EXPANDABLE: &[&str] = &["/review", "/config"];

    if EXPANDABLE.contains(&input) {
        return Some(format!("{input} "));
    }

    if input.starts_with('/')
        && !suggestions.is_empty()
        && EXPANDABLE.contains(&suggestions[selected_idx].slash)
    {
        return Some(format!("{} ", suggestions[selected_idx].slash));
    }

    None
}

/// 명령별 실시간 힌트를 통합 반환한다.
fn realtime_hint(input: &str) -> Option<(Color, String)> {
    review_realtime_hint(input).or_else(|| config_realtime_hint(input))
}

/// `/config` 서브커맨드 실시간 힌트를 반환한다.
fn config_realtime_hint(input: &str) -> Option<(Color, String)> {
    let trimmed = input.trim_start();
    let after = trimmed.strip_prefix("/config")?;

    if !after.is_empty() && !after.starts_with(' ') {
        return None;
    }

    let rest = after.trim();
    if rest.is_empty() {
        return Some((
            Color::Yellow,
            "hint: /config [edit]".to_string(),
        ));
    }

    if rest == "edit" {
        return Some((
            Color::Green,
            "ready: press Enter to open $EDITOR".to_string(),
        ));
    }

    if "edit".starts_with(rest) {
        return Some((
            Color::Yellow,
            "hint: /config [edit]".to_string(),
        ));
    }

    Some((Color::Red, format!("error: unknown subcommand `{rest}`")))
}

/// 서브커맨드 탭 완성을 시도한다.
fn complete_subcommand(input: &str) -> Option<String> {
    let trimmed = input.trim_start();
    let after = trimmed.strip_prefix("/config")?;

    if !after.starts_with(' ') {
        return None;
    }

    let rest = after.trim();
    let subs = ["edit"];
    let matches: Vec<&str> = subs.iter().copied().filter(|s| s.starts_with(rest)).collect();
    if matches.len() == 1 && rest != matches[0] {
        return Some(format!("/config {}", matches[0]));
    }

    None
}

fn review_usage_hint(input: &str) -> Option<&'static str> {
    let trimmed = input.trim_start();
    if trimmed.starts_with("/review") {
        Some("/review <url> [--dry-run] [--force]")
    } else {
        None
    }
}

fn review_realtime_hint(input: &str) -> Option<(Color, String)> {
    let trimmed = input.trim_start();
    if !trimmed.starts_with("/review") {
        return None;
    }

    let rest = trimmed.trim_start_matches("/review").trim();
    if rest.is_empty() {
        return Some((
            Color::Yellow,
            "hint: /review <url> [--dry-run] [--force]".to_string(),
        ));
    }

    let mut url: Option<&str> = None;
    for arg in rest.split_whitespace() {
        match arg {
            "--dry-run" | "--force" => {}
            _ if arg.starts_with("--") => {
                return Some((Color::Red, format!("error: unknown option `{arg}`")));
            }
            _ => {
                if url.is_some() {
                    return Some((
                        Color::Red,
                        "error: only one URL is allowed for /review".to_string(),
                    ));
                }
                url = Some(arg);
            }
        }
    }

    let Some(url) = url else {
        return Some((
            Color::Yellow,
            "hint: /review <url> [--dry-run] [--force]".to_string(),
        ));
    };

    match ReviewTarget::parse(url) {
        Ok(_) => Some((
            Color::Green,
            "ready: valid target URL, press Enter to run".to_string(),
        )),
        Err(_) => Some((
            Color::Red,
            "error: invalid URL (GitHub /pull/<n> or GitLab /-/merge_requests/<iid>)"
                .to_string(),
        )),
    }
}

fn render_frame(
    stdout: &mut io::Stdout,
    input: &str,
    cursor_chars: usize,
    suggestions: &[&Suggestion],
    selected_idx: usize,
) -> Result<()> {
    let (w, h) = terminal::size().unwrap_or((120, 40));
    // 패널 배경의 우측 끊김을 막기 위해 터미널 전체 폭을 사용한다.
    let width = (w as usize).max(20);
    let total_rows = h as usize;

    // 힌트/추천 유무에 따라 패널 높이를 동적으로 결정한다.
    let has_hint = realtime_hint(input).is_some() || review_usage_hint(input).is_some();
    let hint_rows = usize::from(has_hint);
    let suggestion_rows = suggestions.len();
    let extra_rows = hint_rows + suggestion_rows;
    let panel_height = PANEL_BASE_HEIGHT + extra_rows;
    let panel_top = total_rows.saturating_sub(panel_height + PANEL_BOTTOM_PADDING);

    let input_header_row = panel_top;
    let input_row = panel_top + 1;
    let panel_divider_row = panel_top + 2;
    // 하단 구분선 아래부터 힌트·추천을 배치한다.
    let extra_start = panel_divider_row + 1;

    // 이전 프레임 잔상을 지우기 위해 가능한 최대 영역을 클리어한다.
    let max_panel_height = PANEL_BASE_HEIGHT + 1 + SUGGESTIONS.len();
    let clear_top = total_rows.saturating_sub(max_panel_height + PANEL_BOTTOM_PADDING);
    for row in clear_top..total_rows {
        clear_line_at(stdout, row as u16)?;
    }

    // 입력 영역(구분선 + 입력줄 + 구분선)에만 배경색을 적용한다.
    let divider = "─".repeat(width);
    draw_panel_line_at(stdout, input_header_row as u16, &divider, width)?;

    if input.is_empty() {
        let placeholder =
            render_prompt_line("/ Enter run · ↑↓ select · Tab autocomplete", width);
        draw_panel_line_at_with_fg(
            stdout,
            input_row as u16,
            &placeholder,
            width,
            Color::Grey,
        )?;
    } else {
        draw_panel_line_at(
            stdout,
            input_row as u16,
            &render_prompt_line(input, width),
            width,
        )?;
    }

    draw_panel_line_at(stdout, panel_divider_row as u16, &divider, width)?;

    // 하단 구분선 아래: 배경 없이 힌트와 추천을 표시한다.
    let mut next_row = extra_start;

    if let Some((color, line)) = realtime_hint(input) {
        draw_line_at_with_fg(
            stdout,
            next_row as u16,
            &clip_line_display(&line, width),
            width,
            color,
        )?;
        next_row += 1;
    } else if let Some(hint) = review_usage_hint(input) {
        draw_line_at_with_fg(
            stdout,
            next_row as u16,
            &clip_line_display(&format!("hint: {hint}"), width),
            width,
            Color::Yellow,
        )?;
        next_row += 1;
    }

    for (idx, item) in suggestions.iter().enumerate() {
        let marker = if idx == selected_idx { ">" } else { " " };
        draw_line_at_with_fg(
            stdout,
            next_row as u16,
            &clip_line_display(
                &format!(
                    "{marker} {:<10} - {} | usage: {}",
                    item.slash, item.description, item.usage
                ),
                width,
            ),
            width,
            Color::White,
        )?;
        next_row += 1;
    }

    let prompt_cursor_col = prompt_cursor_col(input, cursor_chars, width) as u16;
    execute!(stdout, cursor::MoveTo(prompt_cursor_col, input_row as u16), cursor::Show)?;
    stdout.flush()?;
    Ok(())
}

fn render_prompt_line(input: &str, width: usize) -> String {
    let prefix = "> ";
    let prefix_width = display_width(prefix);
    let available = width.saturating_sub(prefix_width);
    let shown = tail_with_ellipsis_display(input, available);
    clip_line_display(&format!("{prefix}{shown}"), width)
}

fn prompt_cursor_col(input: &str, cursor_chars: usize, width: usize) -> usize {
    let prefix = "> ";
    let prefix_width = display_width(prefix);
    let input_width = display_width(input);
    let before_cursor: String = input.chars().take(cursor_chars).collect();
    let before_cursor_width = display_width(&before_cursor);
    let available = width.saturating_sub(prefix_width);

    if input_width <= available {
        return (prefix_width + before_cursor_width).min(width.saturating_sub(1));
    }

    // 오버플로우 상태에서는 현재 tail 표시 정책상 커서를 입력 끝쪽으로 정렬한다.
    (prefix_width + display_width(&tail_with_ellipsis_display(input, available)))
        .min(width.saturating_sub(1))
}

fn tail_with_ellipsis_display(text: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }

    let text_width = display_width(text);
    if text_width <= max_width {
        return text.to_string();
    }

    if max_width <= 3 {
        return ".".repeat(max_width);
    }

    let target = max_width - 3;
    let mut tail_rev = String::new();
    let mut used = 0usize;

    for ch in text.chars().rev() {
        let cw = char_display_width(ch);
        if used + cw > target {
            break;
        }
        tail_rev.push(ch);
        used += cw;
    }

    let tail: String = tail_rev.chars().rev().collect();
    format!("...{tail}")
}

fn clip_line_display(line: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }

    let width = display_width(line);
    if width <= max_width {
        return line.to_string();
    }

    if max_width <= 3 {
        return ".".repeat(max_width);
    }

    let mut out = String::new();
    let mut used = 0usize;
    let cap = max_width - 3;

    for ch in line.chars() {
        let cw = char_display_width(ch);
        if used + cw > cap {
            break;
        }
        out.push(ch);
        used += cw;
    }

    out.push_str("...");
    out
}

fn pad_line_display(line: &str, width: usize) -> String {
    let mut out = line.to_string();
    let used = display_width(line);
    if used < width {
        out.push_str(&" ".repeat(width - used));
    }
    out
}

fn trim_newline(mut s: String) -> String {
    while matches!(s.chars().last(), Some('\n' | '\r')) {
        s.pop();
    }
    s
}

// 라인을 기본 배경으로 클리어만 한다.
fn clear_line_at(stdout: &mut io::Stdout, row: u16) -> Result<()> {
    execute!(
        stdout,
        cursor::MoveTo(0, row),
        terminal::Clear(ClearType::CurrentLine)
    )?;
    Ok(())
}

// 배경색이 있는 패널 라인 (입력 영역용).
fn draw_panel_line_at(stdout: &mut io::Stdout, row: u16, text: &str, width: usize) -> Result<()> {
    draw_panel_line_at_with_fg(stdout, row, text, width, Color::White)
}

fn draw_panel_line_at_with_fg(
    stdout: &mut io::Stdout,
    row: u16,
    text: &str,
    width: usize,
    fg: Color,
) -> Result<()> {
    execute!(
        stdout,
        cursor::MoveTo(0, row),
        SetBackgroundColor(Color::DarkGrey),
        SetForegroundColor(fg),
        terminal::Clear(ClearType::CurrentLine)
    )?;
    write!(
        stdout,
        "{}",
        pad_line_display(&clip_line_display(text, width), width)
    )?;
    execute!(stdout, ResetColor)?;
    Ok(())
}

// 배경색 없는 일반 라인 (힌트/추천 영역용).
fn draw_line_at_with_fg(
    stdout: &mut io::Stdout,
    row: u16,
    text: &str,
    width: usize,
    fg: Color,
) -> Result<()> {
    execute!(
        stdout,
        cursor::MoveTo(0, row),
        terminal::Clear(ClearType::CurrentLine),
        SetForegroundColor(fg)
    )?;
    write!(
        stdout,
        "{}",
        pad_line_display(&clip_line_display(text, width), width)
    )?;
    execute!(stdout, ResetColor)?;
    Ok(())
}

fn clear_panel_for_output(stdout: &mut io::Stdout) -> Result<()> {
    // 명령 실행 로그는 항상 상단에서 시작하도록 화면을 정리한다.
    execute!(
        stdout,
        cursor::MoveTo(0, 0),
        terminal::Clear(ClearType::All),
        ResetColor,
        cursor::Show
    )?;
    stdout.flush()?;
    Ok(())
}

// unicode-width 크레이트를 사용하여 정확한 터미널 표시 폭을 계산한다.
fn display_width(text: &str) -> usize {
    UnicodeWidthStr::width(text)
}

fn char_display_width(ch: char) -> usize {
    UnicodeWidthChar::width(ch).unwrap_or(0)
}

struct InputGuard;

impl InputGuard {
    fn enter(stdout: &mut io::Stdout) -> Result<Self> {
        terminal::enable_raw_mode()?;
        execute!(stdout, cursor::Show)?;
        Ok(Self)
    }
}

impl Drop for InputGuard {
    fn drop(&mut self) {
        let mut stdout = io::stdout();
        let _ = execute!(stdout, cursor::Show, ResetColor);
        let _ = terminal::disable_raw_mode();
        let _ = stdout.flush();
    }
}

fn insert_char_at(input: &mut String, char_idx: usize, ch: char) {
    let byte_idx = byte_index_at_char(input, char_idx);
    input.insert(byte_idx, ch);
}

fn remove_char_at(input: &mut String, char_idx: usize) {
    let start = byte_index_at_char(input, char_idx);
    let end = byte_index_at_char(input, char_idx + 1);
    if start < end && end <= input.len() {
        input.replace_range(start..end, "");
    }
}

fn byte_index_at_char(input: &str, char_idx: usize) -> usize {
    if char_idx == 0 {
        return 0;
    }
    input
        .char_indices()
        .nth(char_idx)
        .map(|(idx, _)| idx)
        .unwrap_or(input.len())
}
