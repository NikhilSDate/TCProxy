use std::borrow::Cow;

use crossterm::event::{KeyCode, KeyModifiers};
use nu_ansi_term::Style;
use reedline::{ColumnarMenu, default_vi_insert_keybindings, default_vi_normal_keybindings, DefaultCompleter, DefaultHinter, ExampleHighlighter, FileBackedHistory, MenuBuilder, Prompt, Reedline, ReedlineEvent, ReedlineMenu, Signal, Vi};
use reedline::Color;
use serde::{Deserialize, Serialize};

use crate::command::Command;
use crate::error::{Error, Result};

pub fn readline(prompt: Option<String>) -> Result<String> {
    // History
    let history = Box::new(FileBackedHistory::with_file(1024, "history.txt".parse().unwrap()).expect("Failed to create history"));

    // Syntax highlighting
    let commands = Command::all_commands();
    // Tab completion
    let completer = Box::new(DefaultCompleter::new_with_wordlen(commands.clone(), 2));
    // Use the interactive menu to select options from the completer
    let completion_menu = Box::new(ColumnarMenu::default().with_name("completion_menu"));
    // Set up the required keybindings
    let mut keybindings = default_vi_insert_keybindings();
    keybindings.add_binding(
        KeyModifiers::NONE,
        KeyCode::Tab,
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::Menu("completion_menu".to_string()),
            ReedlineEvent::MenuNext,
        ]),
    );

    let edit_mode = Box::new(Vi::new(
        keybindings,
        default_vi_normal_keybindings(),
    ));


    let mut rl = Reedline::create()
        .with_history(history)
        .with_completer(completer)
        .with_menu(ReedlineMenu::EngineCompleter(completion_menu))
        .with_highlighter(Box::new(ExampleHighlighter::new(commands)))
        .with_hinter(Box::new(
            DefaultHinter::default()
                .with_style(Style::new().italic().fg(nu_ansi_term::Color::LightGray))))
        .with_edit_mode(edit_mode);

    let signal = rl.read_line(&mut StandardPrompt::default())?;

    match signal {
        Signal::Success(buffer) => Ok(buffer),
        _ => Err(Error::Signal(signal))
    }
}

// region StandardPrompt
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StandardPrompt {
    left: String,
    right: String,
    indicator: String,
    edit_indicator: String,
    multiline_indicator: String,
    history_search_indicator: String,
    prompt_color: Color,
    prompt_right_color: Color,
    indicator_color: Color,
}

impl StandardPrompt {
    // Setters
    pub fn set_left(&mut self, left: String) {
        self.left = left;
    }
}

impl Default for StandardPrompt {
    fn default() -> Self {
        Self {
            left: String::from("Proxy"),
            right: String::from(""),
            indicator: String::from("(META)> "),
            edit_indicator: String::from("> "),
            multiline_indicator: String::from(">>> "),
            history_search_indicator: String::from("(H)> "),
            prompt_color: Color::Cyan,
            prompt_right_color: Color::Grey,
            indicator_color: Color::Blue,
        }
    }
}

impl Prompt for StandardPrompt {
    // Required
    fn render_prompt_left(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.left)
    }

    fn render_prompt_right(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.right)
    }

    fn render_prompt_indicator(&self, prompt_mode: reedline::PromptEditMode) -> Cow<'_, str> {
        match prompt_mode {
            reedline::PromptEditMode::Default => Cow::Borrowed(&self.edit_indicator),
            reedline::PromptEditMode::Vi(mode) => match mode {
                reedline::PromptViMode::Normal => Cow::Borrowed(&self.indicator),
                reedline::PromptViMode::Insert => Cow::Borrowed(&self.edit_indicator),
            }
            _ => Cow::Borrowed("(?)> "),
        }
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.multiline_indicator)
    }

    fn render_prompt_history_search_indicator(
        &self,
        _history_search: reedline::PromptHistorySearch,
    ) -> Cow<'_, str> {
        Cow::Borrowed(&self.history_search_indicator)
    }

    // Optional
    fn get_prompt_color(&self) -> Color {
        self.prompt_color
    }

    fn get_indicator_color(&self) -> Color {
        self.indicator_color
    }

    fn get_prompt_right_color(&self) -> Color {
        self.prompt_right_color
    }
}
// endregion
