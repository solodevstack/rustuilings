use std::time::Duration;
use std::time::SystemTime;

use color_eyre::Result;
use color_eyre::eyre::Context;
use crossterm::event::{self, KeyCode};
use itertools::Itertools;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Color;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Tabs, Widget};
use ratatui::{DefaultTerminal, Frame};
use strum::{Display, EnumIter, FromRepr, IntoEnumIterator};

use super::tabs::{AboutTab, EmailTab, RecipeTab, TracerouteTab, WeatherTab};
use super::{THEME, destroy};
use crate::app_state::{AppState, ExercisesProgress};
use crate::exercise::{OUTPUT_CAPACITY, RunnableExercise};



#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct App {
    last_file_modified: Option<SystemTime>,
    pub score:          u32,
    pub last_output:    String,
    pub exercise_name:  &'static str,
    pub exercise_path:  &'static str,
    pub n_done:         usize,
    pub n_total:        usize,
    mode:               Mode,
    tab:                Tab,
    about_tab:          AboutTab,
    recipe_tab:         RecipeTab,
    email_tab:          EmailTab,
    traceroute_tab:     TracerouteTab,
    weather_tab:        WeatherTab,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum Mode {
    #[default]
    Running,
    Destroy,
    Quit,
}

#[derive(Debug, Clone, Copy, Default, Display, EnumIter, FromRepr, PartialEq, Eq)]
enum Tab {
    #[default]
    About,
    Recipe,
    Email,
    Traceroute,
    Weather,
}



impl App {
    pub fn new(app_state: &AppState) -> Self {
        let exercise    = app_state.current_exercise();
        let last_output = app_state.last_output().to_string();
        let score       = app_state.game_score();

        Self {
            score,
            last_file_modified: None,
            last_output: last_output.clone(),
            exercise_name: exercise.name,
            exercise_path: exercise.path,
            n_done:  app_state.n_done() as usize,
            n_total: app_state.exercises().len(),
            mode:    Mode::Running,
            tab:     Tab::default(),
            about_tab: AboutTab {
                score,
                last_output,
                ..Default::default()
            },
            recipe_tab:    RecipeTab::default(),
            email_tab:     EmailTab::default(),
            traceroute_tab: TracerouteTab::default(),
            weather_tab:   WeatherTab::default(),
        }
    }



    /// Pull fresh data from AppState every frame and check for file changes.
    fn sync_from(&mut self, app_state: &mut AppState) {
        self.score         = app_state.game_score();
        self.last_output   = app_state.last_output().to_string();
        self.n_done        = app_state.n_done() as usize;
        let exercise       = app_state.current_exercise();
        self.exercise_name = exercise.name;
        self.exercise_path = exercise.path;
        self.about_tab.score       = self.score;
        self.about_tab.last_output = self.last_output.clone();
        self.check_file_changed(app_state);
    }

    /// Poll the exercise file's modification time.
    /// If it changed since last frame, re-run the exercise and update output.
    /// This mirrors how the watch loop detects file saves.
    fn check_file_changed(&mut self, app_state: &mut AppState) {
        use std::fs;

        let path = app_state.current_exercise().path;
        let Ok(metadata) = fs::metadata(path) else { return };
        let Ok(modified) = metadata.modified()  else { return };

        let changed = match self.last_file_modified {
            None       => true,              // first frame — always run once
            Some(last) => modified > last,   // file saved since last frame
        };

        self.last_file_modified = Some(modified);

        if !changed { return; }

        // File was saved — re-run exercise exactly like watch loop does.
        let mut output = Vec::with_capacity(OUTPUT_CAPACITY);
        let _ = app_state
            .current_exercise()
            .run_exercise(Some(&mut output), app_state.cmd_runner());
        app_state.set_last_output(&output);
    }
}



impl App {
    pub fn run(
        mut self,
        mut terminal: DefaultTerminal,
        app_state: &mut AppState,
    ) -> anyhow::Result<()> {
        while self.is_running() {
            self.sync_from(app_state);
            terminal
                .draw(|frame| self.render(frame))
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            self.handle_events(app_state)
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        }
        Ok(())
    }

    fn is_running(&self) -> bool {
        self.mode != Mode::Quit
    }

    fn render(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
        if self.mode == Mode::Destroy {
            destroy::destroy(frame);
        }
    }



    fn handle_events(&mut self, app_state: &mut AppState) -> color_eyre::Result<()> {
        let timeout = Duration::from_secs_f64(1.0 / 50.0);
        if !event::poll(timeout)? {
            return Ok(());
        }
        if let Some(key) = event::read()?.as_key_press_event() {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc    => self.mode = Mode::Quit,
                KeyCode::Char('h') | KeyCode::Left   => self.prev_tab(),
                KeyCode::Char('l') | KeyCode::Right
                | KeyCode::Tab                        => self.next_tab(),
                KeyCode::Char('k') | KeyCode::Up     => self.prev(),
                KeyCode::Char('j') | KeyCode::Down   => self.next(),
                KeyCode::Char('d') | KeyCode::Delete => self.destroy(),

                // 'n' — mirrors WatchEvent::Input(InputEvent::Next)
                // Behaviour is identical to the watch loop:
                //   1. run the exercise
                //   2. if done → advance to next pending
                //   3. if not done → show the error, stay put
                KeyCode::Char('n') => self
                    .try_next_exercise(app_state)
                    .map_err(|e| color_eyre::eyre::eyre!(e.to_string()))?,

                _ => {}
            };
        }
        Ok(())
    }


    fn prev(&mut self) {
        match self.tab {
            Tab::About      => self.about_tab.prev_row(),
            Tab::Recipe     => self.recipe_tab.prev(),
            Tab::Email      => self.email_tab.prev(),
            Tab::Traceroute => self.traceroute_tab.prev_row(),
            Tab::Weather    => self.weather_tab.prev(),
        }
    }

    fn next(&mut self) {
        match self.tab {
            Tab::About      => self.about_tab.next_row(),
            Tab::Recipe     => self.recipe_tab.next(),
            Tab::Email      => self.email_tab.next(),
            Tab::Traceroute => self.traceroute_tab.next_row(),
            Tab::Weather    => self.weather_tab.next(),
        }
    }

    fn prev_tab(&mut self) { self.tab = self.tab.prev(); }
    fn next_tab(&mut self) { self.tab = self.tab.next(); }
    fn destroy(&mut self)  { self.mode = Mode::Destroy; }

 

    // Mirror of `watch_state.next_exercise()` from the watch loop.

    fn try_next_exercise(&mut self, app_state: &mut AppState) -> anyhow::Result<()> {
        // Step 1 — run the exercise, capture output (same as watch loop).
        let mut output = Vec::with_capacity(OUTPUT_CAPACITY);
        let success = app_state
            .current_exercise()
            .run_exercise(Some(&mut output), app_state.cmd_runner())?;

        // Step 2 — always store output so the panel is current.
        app_state.set_last_output(&output);

        if !success {
            // Exercise not done — mirror of ExercisesProgress::CurrentPending.
            // Stay on the current exercise, panel shows the error.
            return Ok(());
        }

        // Step 3 — exercise passed — mirror of done_current_exercise.
        // Clear stale output before advancing.
        app_state.set_last_output(b"");

        let mut stdout = std::io::stdout().lock();
        match app_state.done_current_exercise::<false>(&mut stdout)? {

            // Mirror of ExercisesProgress::AllDone
            ExercisesProgress::AllDone => {
                drop(stdout);
                self.mode = Mode::Quit;
            }

            // Mirror of ExercisesProgress::NewPending
            ExercisesProgress::NewPending => {
                drop(stdout);
                 let editor_handle = app_state.open_editor()?;

                // Reset file modified time so check_file_changed triggers
                // a fresh run on the new exercise next frame.
                self.last_file_modified = None;

                // Run the new exercise immediately so the panel is not blank.
                let mut next_output = Vec::with_capacity(OUTPUT_CAPACITY);
                let _ = app_state
                    .current_exercise()
                    .run_exercise(Some(&mut next_output), app_state.cmd_runner());
                app_state.set_last_output(&next_output);

                  app_state.join_editor_handle(editor_handle)?;
            }

            // Mirror of ExercisesProgress::CurrentPending
            // Shouldn't happen after a passing run but handle gracefully.
            ExercisesProgress::CurrentPending => {
                drop(stdout);
            }
        }

        Ok(())
    }
}


impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let layout = Layout::vertical([
            Constraint::Length(1), // title bar
            Constraint::Min(0),    // tab content
            Constraint::Length(1), // bottom bar
        ]);
        let [title_bar, tab, bottom_bar] = area.layout(&layout);

        Block::new().style(THEME.root).render(area, buf);
        self.render_title_bar(title_bar, buf);
        self.render_selected_tab(tab, buf);
        App::render_bottom_bar(bottom_bar, buf);
    }
}

impl App {
    fn render_title_bar(&self, area: Rect, buf: &mut Buffer) {
        let layout = Layout::horizontal([Constraint::Min(0), Constraint::Length(43)]);
        let [title, tabs] = area.layout(&layout);

        Span::styled("Rustlings Game", THEME.app_title).render(title, buf);
        let titles = Tab::iter().map(Tab::title);
        Tabs::new(titles)
            .style(THEME.tabs)
            .highlight_style(THEME.tabs_selected)
            .select(self.tab as usize)
            .divider("")
            .padding("", "")
            .render(tabs, buf);
    }

    fn render_selected_tab(&self, area: Rect, buf: &mut Buffer) {
        match self.tab {
            Tab::About => {
                let mut tab     = self.about_tab.clone();
                tab.score       = self.score;
                tab.last_output = self.last_output.clone();
                tab.render(area, buf);
            }
            Tab::Recipe     => self.recipe_tab.render(area, buf),
            Tab::Email      => self.email_tab.render(area, buf),
            Tab::Traceroute => self.traceroute_tab.render(area, buf),
            Tab::Weather    => self.weather_tab.render(area, buf),
        }
    }

    fn render_bottom_bar(area: Rect, buf: &mut Buffer) {
        let keys = [
            ("H/←",   "Left"),
            ("L/→",   "Right"),
            ("K/↑",   "Up"),
            ("J/↓",   "Down"),
            ("N",     "Next Exercise"),
            ("D/Del", "Destroy"),
            ("Q/Esc", "Quit"),
        ];
        let spans = keys
            .iter()
            .flat_map(|(key, desc)| {
                let key  = Span::styled(format!(" {key} "),  THEME.key_binding.key);
                let desc = Span::styled(format!(" {desc} "), THEME.key_binding.description);
                [key, desc]
            })
            .collect_vec();
        Line::from(spans)
            .centered()
            .style((Color::Indexed(236), Color::Indexed(232)))
            .render(area, buf);
    }
}


// Tab helpers


impl Tab {
    fn next(self) -> Self {
        let next_index = (self as usize).saturating_add(1);
        Self::from_repr(next_index).unwrap_or(self)
    }

    fn prev(self) -> Self {
        let prev_index = (self as usize).saturating_sub(1);
        Self::from_repr(prev_index).unwrap_or(self)
    }

    fn title(self) -> String {
        match self {
            Self::About => String::new(),
            tab         => format!(" {tab} "),
        }
    }
}