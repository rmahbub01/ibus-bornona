use std::io::{self, IsTerminal, Stdout, Write};
use std::time::Duration;

use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    execute, queue,
    style::{
        Attribute, Color, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor,
    },
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};

use crate::{
    config,
    engine::{SuggestionOrientation, TypingConfig},
    setup,
};

const HOME_ITEMS: [&str; 3] = ["Typing settings", "System health", "Quit"];
const SETTINGS: [(&str, Setting); 8] = [
    ("Automatic vowel forming", Setting::AutoVowel),
    ("Chandrabindu position fix", Setting::AutoChandra),
    ("Old-style reph", Setting::OldReph),
    ("Traditional kar joining", Setting::TraditionalKar),
    ("Word suggestions", Setting::Suggestions),
    ("Space after suggestion", Setting::SuggestionTrailingSpace),
    ("Suggestion learning", Setting::Learning),
    ("Suggestion orientation", Setting::Orientation),
];

#[derive(Clone, Copy)]
enum Setting {
    AutoVowel,
    AutoChandra,
    OldReph,
    TraditionalKar,
    Suggestions,
    SuggestionTrailingSpace,
    Learning,
    Orientation,
}

enum Screen {
    Home(usize),
    Settings(usize),
    Doctor(usize),
}

struct TerminalSession {
    stdout: Stdout,
}

struct Line {
    text: String,
    value: Option<String>,
    selected: bool,
}

impl Line {
    fn text(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            value: None,
            selected: false,
        }
    }

    fn setting(label: &str, value: &str) -> Self {
        Self {
            text: label.into(),
            value: Some(value.into()),
            selected: false,
        }
    }

    fn selected(label: &str, value: &str) -> Self {
        Self {
            text: label.into(),
            value: Some(value.into()),
            selected: true,
        }
    }
}

impl TerminalSession {
    fn new() -> Result<Self, String> {
        terminal::enable_raw_mode().map_err(|error| format!("cannot enable raw mode: {error}"))?;
        let mut stdout = io::stdout();
        if let Err(error) = execute!(stdout, EnterAlternateScreen, Hide) {
            let _ = execute!(stdout, ResetColor, Show, LeaveAlternateScreen);
            let _ = terminal::disable_raw_mode();
            return Err(format!("cannot initialize terminal screen: {error}"));
        }
        Ok(Self { stdout })
    }

    fn draw(&mut self, screen: &Screen, config: &TypingConfig, message: &str) -> Result<(), String> {
        let (width, height) =
            terminal::size().map_err(|error| format!("cannot read terminal size: {error}"))?;
        if width == 0 || height == 0 {
            return Err("terminal reports no usable dimensions; resize terminal and retry".into());
        }
        queue!(self.stdout, MoveTo(0, 0), Clear(ClearType::All)).map_err(write_error)?;

        if width < 64 || height < 24 {
            self.draw_small_terminal(width, height)?;
            return self.stdout.flush().map_err(write_error);
        }

        let panel_width = width.saturating_sub(4).min(76);
        let lines = content(screen, config);
        let panel_height = (lines.len() as u16 + 2).min(height.saturating_sub(7));
        let panel_x = (width - panel_width) / 2;
        let panel_y = ((height - panel_height) / 2).max(3);

        self.draw_header(width)?;
        self.draw_box(panel_x, panel_y, panel_width, panel_height)?;

        let visible_lines = panel_height.saturating_sub(2) as usize;
        let first_line = match screen {
            Screen::Doctor(offset) => {
                (*offset).min(lines.len().saturating_sub(visible_lines))
            }
            _ => 0,
        };
        for (index, line) in lines
            .iter()
            .skip(first_line)
            .take(visible_lines)
            .enumerate()
        {
            let row = panel_y + 1 + index as u16;
            self.draw_line(panel_x + 2, row, panel_width - 4, line)?;
        }

        self.draw_footer(width, height, screen, message)?;
        self.stdout.flush().map_err(write_error)
    }

    fn draw_header(&mut self, width: u16) -> Result<(), String> {
        let title = "BORNONA";
        let title_x = width.saturating_sub(title.len() as u16) / 2;
        queue!(
            self.stdout,
            MoveTo(title_x, 0),
            SetForegroundColor(Color::Cyan),
            SetAttribute(Attribute::Bold)
        )
        .map_err(write_error)?;
        write!(self.stdout, "{title}").map_err(write_error)?;

        let subtitle = "BENGALI INPUT TOOLKIT";
        let subtitle_x = width.saturating_sub(subtitle.len() as u16) / 2;
        queue!(
            self.stdout,
            MoveTo(subtitle_x, 1),
            SetForegroundColor(Color::DarkGrey),
            SetAttribute(Attribute::Reset)
        )
        .map_err(write_error)?;
        write!(self.stdout, "{subtitle}").map_err(write_error)?;
        queue!(self.stdout, ResetColor).map_err(write_error)
    }

    fn draw_box(&mut self, x: u16, y: u16, width: u16, height: u16) -> Result<(), String> {
        queue!(self.stdout, SetForegroundColor(Color::DarkCyan)).map_err(write_error)?;
        self.write_at(x, y, &format!("╭{}╮", "─".repeat(width.saturating_sub(2) as usize)))?;
        for row in 1..height.saturating_sub(1) {
            self.write_at(x, y + row, "│")?;
            self.write_at(x + width - 1, y + row, "│")?;
        }
        self.write_at(
            x,
            y + height - 1,
            &format!("╰{}╯", "─".repeat(width.saturating_sub(2) as usize)),
        )?;
        queue!(self.stdout, ResetColor).map_err(write_error)
    }

    fn draw_line(&mut self, x: u16, y: u16, width: u16, line: &Line) -> Result<(), String> {
        let text_width = line
            .text
            .chars()
            .count()
            .min(width.saturating_sub(2) as usize);
        if line.selected {
            queue!(
                self.stdout,
                MoveTo(x, y),
                SetForegroundColor(Color::Black),
                SetBackgroundColor(Color::Cyan),
                SetAttribute(Attribute::Bold)
            )
            .map_err(write_error)?;
            write!(self.stdout, " {:<width$}", line.text.chars().take(text_width).collect::<String>(), width = width.saturating_sub(2) as usize)
                .map_err(write_error)?;
            if let Some(value) = &line.value {
                let value_width = value.chars().count().min(width as usize / 2);
                let value_x = x + width.saturating_sub(value_width as u16 + 2);
                queue!(self.stdout, MoveTo(value_x, y)).map_err(write_error)?;
                write!(self.stdout, "{value} ").map_err(write_error)?;
            }
        } else {
            queue!(
                self.stdout,
                MoveTo(x + 1, y),
                SetForegroundColor(Color::White),
                SetAttribute(Attribute::Reset)
            )
            .map_err(write_error)?;
            write!(self.stdout, "{}", line.text.chars().take(text_width).collect::<String>())
                .map_err(write_error)?;
            if let Some(value) = &line.value {
                let value_width = value.chars().count();
                let value_x = x + width.saturating_sub(value_width as u16 + 2);
                queue!(
                    self.stdout,
                    MoveTo(value_x, y),
                    SetForegroundColor(Color::Green)
                )
                .map_err(write_error)?;
                write!(self.stdout, "{value}").map_err(write_error)?;
            }
        }
        queue!(self.stdout, ResetColor, SetAttribute(Attribute::Reset)).map_err(write_error)
    }

    fn draw_footer(
        &mut self,
        width: u16,
        height: u16,
        screen: &Screen,
        message: &str,
    ) -> Result<(), String> {
        let text = if !message.is_empty() {
            message
        } else {
            match screen {
                Screen::Home(_) => "↑/↓ Move · Enter Open · Q Quit",
                Screen::Settings(_) => "↑/↓ Move · Enter Toggle · ←/→ Orientation · Esc Back · q Quit",
                Screen::Doctor(_) => "↑/↓ Scroll · Enter/Esc Back · q Quit",
            }
        };
        let text_width = text.chars().count().min(width.saturating_sub(2) as usize);
        let x = width.saturating_sub(text_width as u16) / 2;
        queue!(
            self.stdout,
            MoveTo(0, height - 2),
            SetForegroundColor(Color::DarkGrey)
        )
        .map_err(write_error)?;
        write!(
            self.stdout,
            "{:width$}",
            "",
            width = width.saturating_sub(1) as usize
        )
        .map_err(write_error)?;
        queue!(self.stdout, MoveTo(x, height - 2), SetForegroundColor(Color::Grey))
            .map_err(write_error)?;
        write!(self.stdout, "{text}").map_err(write_error)?;
        queue!(self.stdout, ResetColor).map_err(write_error)
    }

    fn draw_small_terminal(&mut self, width: u16, height: u16) -> Result<(), String> {
        let message = "Resize terminal to at least 64 x 24";
        let text = message.chars().take(width as usize).collect::<String>();
        let x = width.saturating_sub(text.chars().count() as u16) / 2;
        let y = height / 2;
        queue!(
            self.stdout,
            MoveTo(x, y),
            SetForegroundColor(Color::Yellow),
            SetAttribute(Attribute::Bold)
        )
        .map_err(write_error)?;
        write!(self.stdout, "{text}").map_err(write_error)?;
        queue!(self.stdout, ResetColor, SetAttribute(Attribute::Reset)).map_err(write_error)
    }

    fn write_at(&mut self, x: u16, y: u16, text: &str) -> Result<(), String> {
        queue!(self.stdout, MoveTo(x, y)).map_err(write_error)?;
        write!(self.stdout, "{text}").map_err(write_error)
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = execute!(self.stdout, ResetColor);
        let _ = execute!(self.stdout, Show);
        let _ = execute!(self.stdout, LeaveAlternateScreen);
        let _ = terminal::disable_raw_mode();
    }
}

pub fn run() -> Result<(), String> {
    if !io::stdin().is_terminal() {
        return Err("TUI requires an interactive terminal".into());
    }
    let mut config = config::load()?;
    let mut terminal = TerminalSession::new()?;
    let mut screen = Screen::Home(0);
    let mut message = String::new();
    let mut last_size =
        terminal::size().map_err(|error| format!("cannot read terminal size: {error}"))?;
    let mut redraw = true;

    loop {
        if redraw {
            terminal.draw(&screen, &config, &message)?;
            message.clear();
            redraw = false;
        }

        if event::poll(Duration::from_millis(200))
            .map_err(|error| format!("cannot poll terminal events: {error}"))?
        {
            match event::read().map_err(|error| format!("cannot read key: {error}"))? {
                Event::Resize(_, _) => {
                    last_size = terminal::size()
                        .map_err(|error| format!("cannot read terminal size: {error}"))?;
                    redraw = true;
                }
                Event::Key(key) if key.kind != KeyEventKind::Release => {
                    if should_quit(key) {
                        break;
                    }
                    if handle_key(key, &mut screen, &mut config, &mut message) {
                        break;
                    }
                    redraw = true;
                }
                _ => {}
            }
        } else {
            let size = terminal::size()
                .map_err(|error| format!("cannot read terminal size: {error}"))?;
            if size != last_size {
                last_size = size;
                redraw = true;
            }
        }
    }
    Ok(())
}

fn should_quit(key: KeyEvent) -> bool {
    key.code == KeyCode::Char('q')
}

fn handle_key(
    key: KeyEvent,
    screen: &mut Screen,
    config: &mut TypingConfig,
    message: &mut String,
) -> bool {
    match screen {
        Screen::Home(selected) => match key.code {
            KeyCode::Up => *selected = selected.saturating_sub(1),
            KeyCode::Down => *selected = (*selected + 1).min(HOME_ITEMS.len() - 1),
            KeyCode::Enter => match *selected {
                0 => *screen = Screen::Settings(0),
                1 => *screen = Screen::Doctor(0),
                _ => return true,
            },
            KeyCode::Esc => {}
            _ => {}
        },
        Screen::Settings(selected) => match key.code {
            KeyCode::Esc => *screen = Screen::Home(0),
            KeyCode::Up => *selected = selected.saturating_sub(1),
            KeyCode::Down => *selected = (*selected + 1).min(SETTINGS.len() - 1),
            KeyCode::Enter | KeyCode::Char(' ') | KeyCode::Left | KeyCode::Right => {
                let setting = SETTINGS[*selected].1;
                if let Some(reverse) = setting_action(setting, key.code) {
                    match update_setting(setting, reverse, config) {
                        Ok(()) => *message = "Setting saved.".into(),
                        Err(error) => *message = format!("Could not save setting: {error}"),
                    }
                }
            }
            _ => {}
        },
        Screen::Doctor(offset) => match key.code {
            KeyCode::Up => *offset = offset.saturating_sub(1),
            KeyCode::Down => *offset = offset.saturating_add(1),
            KeyCode::Esc | KeyCode::Enter => *screen = Screen::Home(1),
            _ => {}
        },
    }
    false
}

fn setting_action(setting: Setting, key: KeyCode) -> Option<bool> {
    match key {
        KeyCode::Enter | KeyCode::Char(' ') => Some(false),
        KeyCode::Right if matches!(setting, Setting::Orientation) => Some(false),
        KeyCode::Left if matches!(setting, Setting::Orientation) => Some(true),
        _ => None,
    }
}

fn content(screen: &Screen, config: &TypingConfig) -> Vec<Line> {
    match screen {
        Screen::Home(selected) => {
            let mut lines = vec![
                Line::text("Welcome"),
                Line::text("Choose a tool to get started."),
                Line::text(""),
            ];
            lines.extend(HOME_ITEMS.iter().enumerate().map(|(index, item)| {
                if index == *selected {
                    Line::selected(&format!("›  {item}"), "")
                } else {
                    Line::text(format!("   {item}"))
                }
            }));
            lines
        }
        Screen::Settings(selected) => {
            let mut lines = vec![
                Line::text("Typing settings"),
                Line::text("Changes save immediately."),
                Line::text(""),
            ];
            lines.extend(SETTINGS.iter().enumerate().map(|(index, (label, setting))| {
                let value = setting_value(*setting, config);
                if index == *selected {
                    Line::selected(&format!("›  {label}"), value)
                } else {
                    Line::setting(&format!("   {label}"), value)
                }
            }));
            lines
        }
        Screen::Doctor(_) => {
            let mut lines = vec![
                Line::text("System health"),
                Line::text("Installation and desktop integration status."),
                Line::text(""),
            ];
            lines.extend(setup::doctor().lines().map(Line::text));
            lines
        }
    }
}

fn setting_value(setting: Setting, config: &TypingConfig) -> &'static str {
    match setting {
        Setting::AutoVowel => on_off(config.auto_vowel),
        Setting::AutoChandra => on_off(config.auto_chandra),
        Setting::OldReph => on_off(config.old_reph),
        Setting::TraditionalKar => on_off(config.traditional_kar),
        Setting::Suggestions => on_off(config.word_suggestion),
        Setting::SuggestionTrailingSpace => on_off(config.suggestion_trailing_space),
        Setting::Learning => on_off(config.learning),
        Setting::Orientation => config.suggestion_orientation.as_str(),
    }
}

fn on_off(enabled: bool) -> &'static str {
    if enabled { "ON" } else { "OFF" }
}

fn update_setting(
    setting: Setting,
    reverse: bool,
    config: &mut TypingConfig,
) -> Result<(), String> {
    let updated = toggled_setting(setting, reverse, config);
    config::save(&updated)?;
    *config = updated;
    Ok(())
}

fn toggled_setting(setting: Setting, reverse: bool, config: &TypingConfig) -> TypingConfig {
    let mut updated = config.clone();
    match setting {
        Setting::AutoVowel => updated.auto_vowel = !updated.auto_vowel,
        Setting::AutoChandra => updated.auto_chandra = !updated.auto_chandra,
        Setting::OldReph => updated.old_reph = !updated.old_reph,
        Setting::TraditionalKar => updated.traditional_kar = !updated.traditional_kar,
        Setting::Suggestions => updated.word_suggestion = !updated.word_suggestion,
        Setting::SuggestionTrailingSpace => {
            updated.suggestion_trailing_space = !updated.suggestion_trailing_space
        }
        Setting::Learning => updated.learning = !updated.learning,
        Setting::Orientation => {
            updated.suggestion_orientation = match updated.suggestion_orientation {
                SuggestionOrientation::Horizontal if reverse => SuggestionOrientation::System,
                SuggestionOrientation::Horizontal => SuggestionOrientation::Vertical,
                SuggestionOrientation::Vertical if reverse => SuggestionOrientation::Horizontal,
                SuggestionOrientation::Vertical => SuggestionOrientation::System,
                SuggestionOrientation::System if reverse => SuggestionOrientation::Vertical,
                SuggestionOrientation::System => SuggestionOrientation::Horizontal,
            }
        }
    }
    updated
}

fn write_error(error: io::Error) -> String {
    format!("cannot write terminal screen: {error}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    #[test]
    fn orientation_actions_move_in_both_directions() {
        let mut config = TypingConfig::default();
        assert_eq!(
            setting_action(Setting::AutoVowel, KeyCode::Left),
            None
        );
        assert_eq!(
            setting_action(Setting::Orientation, KeyCode::Left),
            Some(true)
        );
        assert_eq!(
            setting_action(Setting::Orientation, KeyCode::Right),
            Some(false)
        );
        config = toggled_setting(Setting::Orientation, false, &config);
        assert_eq!(config.suggestion_orientation, SuggestionOrientation::Vertical);
        config = toggled_setting(Setting::Orientation, true, &config);
        assert_eq!(config.suggestion_orientation, SuggestionOrientation::Horizontal);
    }

    #[test]
    fn suggestion_trailing_space_setting_toggles() {
        let config = TypingConfig::default();
        assert_eq!(
            setting_value(Setting::SuggestionTrailingSpace, &config),
            "OFF"
        );
        let updated = toggled_setting(Setting::SuggestionTrailingSpace, false, &config);
        assert!(updated.suggestion_trailing_space);
    }

    #[test]
    fn only_current_menu_item_is_highlighted() {
        let config = TypingConfig::default();
        let items = content(&Screen::Settings(3), &config);
        assert_eq!(items.iter().filter(|line| line.selected).count(), 1);
        assert!(items[6].selected);
    }

    #[test]
    fn home_menu_contains_only_settings_health_and_quit() {
        assert_eq!(
            HOME_ITEMS,
            ["Typing settings", "System health", "Quit"]
        );
        assert!(should_quit(
            KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE)
        ));
    }

    #[test]
    fn quit_menu_item_exits() {
        let mut screen = Screen::Home(HOME_ITEMS.len() - 1);
        assert!(handle_key(
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
            &mut screen,
            &mut TypingConfig::default(),
            &mut String::new(),
        ));
    }

    #[test]
    fn doctor_results_scroll_in_both_directions() {
        let mut screen = Screen::Doctor(0);
        let mut config = TypingConfig::default();
        let mut message = String::new();
        assert!(!handle_key(
            KeyEvent::new(KeyCode::Down, KeyModifiers::NONE),
            &mut screen,
            &mut config,
            &mut message,
        ));
        assert!(matches!(screen, Screen::Doctor(1)));
        handle_key(
            KeyEvent::new(KeyCode::Up, KeyModifiers::NONE),
            &mut screen,
            &mut config,
            &mut message,
        );
        assert!(matches!(screen, Screen::Doctor(0)));
    }
}
