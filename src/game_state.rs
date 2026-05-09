//! Game state for Rustlings gamify mode.
//!
//! Tracks a randomly selected exercise, score, and progress.
//! The user cannot advance until the current exercise is solved.
//! Each completed exercise awards 10 points.
//!
//! State is persisted in `.rustlings-game-state.txt` so progress
//! survives restarts.

use anyhow::{Context, Result, bail};
use rand::seq::IndexedRandom;
use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, Write},
};

use crate::exercise::Exercise;

pub const GAME_STATE_FILE_NAME: &str = ".rustlings-game-state.txt";
const GAME_STATE_FILE_HEADER: &[u8] = b"DON'T EDIT THIS FILE! (game state)\n\n";
const POINTS_PER_EXERCISE: u32 = 10;

// ---------------------------------------------------------------------------
// GameState
// ---------------------------------------------------------------------------

/// Persistent game state.
///
/// Layout of `.rustlings-game-state.txt`:
/// ```text
/// DON'T EDIT THIS FILE! (game state)
///                                       <- empty line
/// <score>                               <- line 3: u32 score
/// <current_exercise_name>               <- line 4: name of locked exercise
/// <done_exercise_name>                  <- line 5+: one per line
/// ...
/// ```
/// 

pub struct GameState {
    /// Current score (10 pts per exercise).
    pub score: u32,

    /// Index into `exercises` of the currently locked exercise.
    pub current_exercise_ind: usize,

    /// Snapshot of all exercises (borrowed from AppState).
    exercises: Vec<GameExercise>,

    /// Open handle to the state file for writing.
    state_file: File,

    /// Reusable write buffer.
    file_buf: Vec<u8>,
}

/// Lightweight per-exercise record stored inside `GameState`.
#[derive(Debug,Clone)]
pub struct GameExercise {
    pub name:  &'static str,
    pub path:  &'static str,
    pub done:  bool,
}
impl std::fmt::Debug for GameState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GameState")
            .field("score", &self.score)
            .field("current_exercise_ind", &self.current_exercise_ind)
            .field("exercises", &self.exercises)
            .finish_non_exhaustive() // skips state_file and file_buf
    }
}

impl GameState {
    // ── Constructor ──────────────────────────────────────────────────────────

    /// Build `GameState` from the exercises already loaded by `AppState`.
    ///
    /// - If a saved game file exists it is restored.
    /// - Otherwise a random pending exercise is picked and the file is created.
    pub fn new(exercises: &[Exercise]) -> Result<Self> {
        let mut state_file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(GAME_STATE_FILE_NAME)
            .with_context(|| {
                format!("Failed to open or create the game state file {GAME_STATE_FILE_NAME}")
            })?;

        let mut game_exercises: Vec<GameExercise> = exercises
            .iter()
            .map(|e| GameExercise {
                name: e.name,
                path: e.path,
                done: e.done,
            })
            .collect();

        let mut score               = 0u32;
        let mut current_exercise_ind = None::<usize>;
        let mut file_buf            = Vec::with_capacity(2048);

        // ── Try to restore existing game ─────────────────────────────────────
        let restored = 'restore: {
            if state_file.read_to_end(&mut file_buf).is_err() {
                break 'restore false;
            }

            // Skip the two-line header.
            let mut lines = file_buf.split(|b| *b == b'\n').skip(2);

            // Line 3: score
            let score_line = match lines.next() {
                Some(l) if !l.is_empty() => l,
                _ => break 'restore false,
            };
            let Ok(score_str) = std::str::from_utf8(score_line) else {
                break 'restore false;
            };
            let Ok(saved_score) = score_str.trim().parse::<u32>() else {
                break 'restore false;
            };

            // Line 4: current exercise name
            let cur_name_line = match lines.next() {
                Some(l) if !l.is_empty() => l,
                _ => break 'restore false,
            };
            let Ok(cur_name) = std::str::from_utf8(cur_name_line) else {
                break 'restore false;
            };
            let cur_name = cur_name.trim();

            // Remaining lines: done exercise names
            let mut done_names = std::collections::HashSet::new();
            for line in lines {
                if line.is_empty() { break; }
                done_names.insert(line);
            }

            // Apply done flags and find the current exercise index.
            let mut found_ind = None;
            for (ind, ex) in game_exercises.iter_mut().enumerate() {
                if done_names.contains(ex.name.as_bytes()) {
                    ex.done = true;
                }
                if ex.name == cur_name {
                    found_ind = Some(ind);
                }
            }

            let Some(ind) = found_ind else {
                break 'restore false;
            };

            score                = saved_score;
            current_exercise_ind = Some(ind);
            true
        };

        // ── Fresh start: pick a random pending exercise ───────────────────────
        if !restored {
            let pending: Vec<usize> = game_exercises
                .iter()
                .enumerate()
                .filter(|(_, e)| !e.done)
                .map(|(i, _)| i)
                .collect();

            if pending.is_empty() {
                bail!("All exercises are already done — nothing left to play!");
            }

            let mut rng = rand::rng();
            let &picked = pending
                .choose(&mut rng)
                .context("Failed to pick a random exercise")?;

            current_exercise_ind = Some(picked);
        }

        file_buf.clear();
        file_buf.extend_from_slice(GAME_STATE_FILE_HEADER);

        let mut slf = Self {
            score,
            current_exercise_ind: current_exercise_ind
                .context("No exercise index was resolved")?,
            exercises: game_exercises,
            state_file,
            file_buf,
        };

        // Always write on construction so the file reflects the initial state.
        slf.write()?;

        Ok(slf)
    }

    // ── Accessors ────────────────────────────────────────────────────────────

    pub fn current_exercise(&self) -> &GameExercise {
        &self.exercises[self.current_exercise_ind]
    }

    pub fn exercises(&self) -> &[GameExercise] {
        &self.exercises
    }

    pub fn n_done(&self) -> usize {
        self.exercises.iter().filter(|e| e.done).count()
    }

    pub fn n_total(&self) -> usize {
        self.exercises.len()
    }

    pub fn n_pending(&self) -> usize {
        self.exercises.iter().filter(|e| !e.done).count()
    }

    pub fn is_exercise_locked(&self, exercise_name: &str) -> bool {
        self.current_exercise().name != exercise_name
    }

    // ── Progress ─────────────────────────────────────────────────────────────

    /// Mark the current exercise as done, award points, then pick the next
    /// random pending exercise.
    ///
    /// Returns `true` when all exercises are complete.
    pub fn complete_current_exercise(&mut self) -> Result<bool> {
        // Mark done and award points.
        let ind = self.current_exercise_ind;
        if !self.exercises[ind].done {
            self.exercises[ind].done = true;
            self.score += POINTS_PER_EXERCISE;
        }

        // Find remaining pending exercises (excluding the one just done).
        let pending: Vec<usize> = self
            .exercises
            .iter()
            .enumerate()
            .filter(|(i, e)| *i != ind && !e.done)
            .map(|(i, _)| i)
            .collect();

        if pending.is_empty() {
            self.write()?;
            return Ok(true); // all done
        }

        // Pick a new random pending exercise.
        let mut rng = rand::rng();
        let &next_ind = pending
            .choose(&mut rng)
            .context("Failed to pick the next random exercise")?;

        self.current_exercise_ind = next_ind;
        self.write()?;

        Ok(false)
    }

    /// Reset the game: clear all done flags, reset score, pick a fresh random exercise.
    pub fn reset(&mut self) -> Result<()> {
        for ex in &mut self.exercises {
            ex.done = false;
        }
        self.score = 0;

        let pending: Vec<usize> = (0..self.exercises.len()).collect();
        let mut rng = rand::rng();
        let &picked = pending
            .choose(&mut rng)
            .context("No exercises available to reset to")?;

        self.current_exercise_ind = picked;
        self.write()
    }

    // ── Persistence ──────────────────────────────────────────────────────────

    fn write(&mut self) -> Result<()> {
        self.file_buf.truncate(GAME_STATE_FILE_HEADER.len());

        // Score line.
        let score_str = self.score.to_string();
        self.file_buf.extend_from_slice(score_str.as_bytes());
        self.file_buf.push(b'\n');

        // Current exercise name.
        self.file_buf
            .extend_from_slice(self.exercises[self.current_exercise_ind].name.as_bytes());
        self.file_buf.push(b'\n');

        // Done exercise names.
        for ex in &self.exercises {
            if ex.done {
                self.file_buf.extend_from_slice(ex.name.as_bytes());
                self.file_buf.push(b'\n');
            }
        }

        self.state_file
            .rewind()
            .with_context(|| format!("Failed to rewind {GAME_STATE_FILE_NAME}"))?;
        self.state_file
            .set_len(0)
            .with_context(|| format!("Failed to truncate {GAME_STATE_FILE_NAME}"))?;
        self.state_file
            .write_all(&self.file_buf)
            .with_context(|| format!("Failed to write {GAME_STATE_FILE_NAME}"))?;

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// main.rs integration helper
// ---------------------------------------------------------------------------

/// Called from `main.rs` inside `Some(Command::Rustuigames { name })`.
///
/// `name` is currently unused — the game always picks a random exercise.
/// You can use `name` later to seed a specific exercise if desired.
pub fn start_game(exercises: &[Exercise], _name: Option<&str>) -> Result<()> {
    let mut game_state = GameState::new(exercises)?;

    println!("🦀 Welcome to Rustlings Game Mode!");
    println!("════════════════════════════════════");
    println!(
        "  Score  : {} pts",
        game_state.score
    );
    println!(
        "  Progress: {}/{} exercises done",
        game_state.n_done(),
        game_state.n_total()
    );
    println!("════════════════════════════════════");

    let cur = game_state.current_exercise();
    println!("\n📂 Your exercise: {}", cur.path);
    println!("   Fix it and run `rustlings` to check.");
    println!("\n  💡 Rules:");
    println!("   • You cannot skip to a different exercise.");
    println!("   • Complete this one to unlock the next random exercise.");
    println!("   • Each completed exercise = {} pts.", POINTS_PER_EXERCISE);

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // Build a minimal Exercise slice for testing without touching the filesystem.
    fn make_exercises(names: &[&'static str]) -> Vec<Exercise> {
        names
            .iter()
            .map(|&name| Exercise {
                name,
                dir: None,
                path: "exercises/dummy.rs",
                canonical_path: None,
                test: false,
                strict_clippy: false,
                hint: "",
                done: false,
            })
            .collect()
    }

    // Build a GameState backed by a temporary file, bypassing `new()` so we
    // don't need a real filesystem layout.
    fn make_game_state(exercises: Vec<Exercise>, start_ind: usize) -> GameState {
        let game_exercises = exercises
            .iter()
            .map(|e| GameExercise { name: e.name, path: e.path, done: e.done })
            .collect();

        GameState {
            score: 0,
            current_exercise_ind: start_ind,
            exercises: game_exercises,
            state_file: tempfile::tempfile().unwrap(),
            file_buf: Vec::new(),
        }
    }

    // ── GameExercise accessors ───────────────────────────────────────────────

    #[test]
    fn current_exercise_returns_correct_exercise() {
        let exs = make_exercises(&["alpha", "beta", "gamma"]);
        let gs  = make_game_state(exs, 1);
        assert_eq!(gs.current_exercise().name, "beta");
    }

    #[test]
    fn n_done_starts_at_zero() {
        let exs = make_exercises(&["a", "b", "c"]);
        let gs  = make_game_state(exs, 0);
        assert_eq!(gs.n_done(), 0);
    }

    #[test]
    fn n_total_matches_exercise_count() {
        let exs = make_exercises(&["a", "b", "c"]);
        let gs  = make_game_state(exs, 0);
        assert_eq!(gs.n_total(), 3);
    }

    #[test]
    fn n_pending_equals_total_minus_done() {
        let mut exs = make_exercises(&["a", "b", "c"]);
        exs[0].done = true;
        let game_exercises: Vec<GameExercise> = exs
            .iter()
            .map(|e| GameExercise { name: e.name, path: e.path, done: e.done })
            .collect();
        let gs = GameState {
            score: 0,
            current_exercise_ind: 1,
            exercises: game_exercises,
            state_file: tempfile::tempfile().unwrap(),
            file_buf: Vec::new(),
        };
        assert_eq!(gs.n_pending(), 2);
        assert_eq!(gs.n_done(), 1);
    }

    // ── is_exercise_locked ───────────────────────────────────────────────────

    // #[test]
    // fn current_exercise_is_not_locked() {
    //     let exs = make_exercises(&["a", "b", "c"]);
    //     let gs  = make_game_state(exs, 2);
    //     assert!(!gs.is_exercise_locked("gamma"));
    // }
    #[test]
fn current_exercise_is_not_locked() {
    let exs = make_exercises(&["a", "b", "c"]);
    let gs  = make_game_state(exs, 2);
    // index 2 = "c", not "gamma"
    assert!(!gs.is_exercise_locked("c"));
}

    // #[test]
    // fn other_exercises_are_locked() {
    //     let exs = make_exercises(&["a", "b", "c"]);
    //     let gs  = make_game_state(exs, 0);
    //     assert!(gs.is_exercise_locked("beta"));
    //     assert!(gs.is_exercise_locked("gamma"));
    // }
    #[test]
fn other_exercises_are_locked() {
    let exs = make_exercises(&["a", "b", "c"]);
    let gs  = make_game_state(exs, 0);
    // index 0 = "a", so "b" and "c" are locked
    assert!(gs.is_exercise_locked("b"));
    assert!(gs.is_exercise_locked("c"));
}

    // ── complete_current_exercise ────────────────────────────────────────────

    #[test]
    fn complete_awards_10_points() {
        let exs = make_exercises(&["a", "b", "c"]);
        let mut gs = make_game_state(exs, 0);
        let all_done = gs.complete_current_exercise().unwrap();
        assert!(!all_done);
        assert_eq!(gs.score, POINTS_PER_EXERCISE);
    }

    #[test]
    fn complete_marks_exercise_done() {
        let exs = make_exercises(&["a", "b"]);
        let mut gs = make_game_state(exs, 0);
        gs.complete_current_exercise().unwrap();
        assert!(gs.exercises[0].done);
    }

    #[test]
    fn complete_does_not_double_award_points() {
        let exs = make_exercises(&["a", "b"]);
        let mut gs = make_game_state(exs, 0);
        gs.complete_current_exercise().unwrap();
        // Force calling complete again on already-done exercise.
        gs.current_exercise_ind = 0;
        gs.complete_current_exercise().unwrap();
        assert_eq!(gs.score, POINTS_PER_EXERCISE); // still 10, not 20
    }

    #[test]
    fn complete_last_exercise_returns_all_done() {
        let exs = make_exercises(&["only"]);
        let mut gs = make_game_state(exs, 0);
        let all_done = gs.complete_current_exercise().unwrap();
        assert!(all_done);
    }

    #[test]
    fn complete_moves_to_a_different_pending_exercise() {
        let exs = make_exercises(&["a", "b", "c"]);
        let mut gs = make_game_state(exs, 0);
        let prev_ind = gs.current_exercise_ind;
        gs.complete_current_exercise().unwrap();
        // The new exercise must not be the one just completed.
        assert_ne!(gs.current_exercise_ind, prev_ind);
        assert!(!gs.exercises[gs.current_exercise_ind].done);
    }

    #[test]
    fn score_accumulates_across_completions() {
        let exs = make_exercises(&["a", "b", "c"]);
        let mut gs = make_game_state(exs, 0);

        // Complete exercises until all done.
        for _ in 0..3 {
            let all_done = gs.complete_current_exercise().unwrap();
            if all_done { break; }
        }

        assert_eq!(gs.score, 3 * POINTS_PER_EXERCISE);
    }

    // ── reset ────────────────────────────────────────────────────────────────

    #[test]
    fn reset_clears_score() {
        let exs = make_exercises(&["a", "b"]);
        let mut gs = make_game_state(exs, 0);
        gs.score = 50;
        gs.reset().unwrap();
        assert_eq!(gs.score, 0);
    }

    #[test]
    fn reset_clears_done_flags() {
        let mut exs = make_exercises(&["a", "b", "c"]);
        exs[0].done = true;
        exs[1].done = true;
        let game_exercises: Vec<GameExercise> = exs
            .iter()
            .map(|e| GameExercise { name: e.name, path: e.path, done: e.done })
            .collect();
        let mut gs = GameState {
            score: 20,
            current_exercise_ind: 2,
            exercises: game_exercises,
            state_file: tempfile::tempfile().unwrap(),
            file_buf: Vec::new(),
        };
        gs.reset().unwrap();
        assert!(gs.exercises.iter().all(|e| !e.done));
    }

    #[test]
    fn reset_picks_a_valid_exercise() {
        let exs = make_exercises(&["a", "b", "c"]);
        let mut gs = make_game_state(exs, 0);
        gs.reset().unwrap();
        assert!(gs.current_exercise_ind < gs.n_total());
    }

    // ── Points constant ──────────────────────────────────────────────────────

    #[test]
    fn points_per_exercise_is_10() {
        assert_eq!(POINTS_PER_EXERCISE, 10);
    }

    // ── n_done / n_pending consistency ──────────────────────────────────────

    #[test]
    fn done_plus_pending_equals_total() {
        let exs = make_exercises(&["a", "b", "c", "d"]);
        let mut gs = make_game_state(exs, 0);
        gs.complete_current_exercise().unwrap();
        assert_eq!(gs.n_done() + gs.n_pending(), gs.n_total());
    }
}