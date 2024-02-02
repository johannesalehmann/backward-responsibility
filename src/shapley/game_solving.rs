use crate::game::{Game, Player};
use crate::shapley::is_subset_of;
use crate::transition_systems::TransitionSystem;
use indicatif::ProgressBar;
use rayon::prelude::*;

struct SolverThreadState {
    game: Game,
    new_minimal_coalitions: Vec<u64>,
}

impl SolverThreadState {
    pub fn new(game: Game) -> Self {
        Self {
            game,
            new_minimal_coalitions: Vec::new(),
        }
    }
}

pub enum StateGroups {
    Individual { state_indices: Vec<usize> },
    Grouped { groups: Vec<StateGroup> },
}

pub struct StateGroup {
    name: String,
    members: Vec<usize>,
}

impl StateGroups {
    pub fn individual_from_game(game: &Game) -> Self {
        Self::Individual {
            state_indices: game.get_significant_states(),
        }
    }
    pub fn individual_on_path(game: &Game) -> Self {
        let mut state_indices = Vec::new();
        for i in 0..game.states.len() {
            if game.states[i].default_owner == Player::Path {
                state_indices.push(i);
            }
        }

        Self::Individual { state_indices }
    }

    pub fn grouped_by_label_from_game(game: &Game) -> Self {
        let mut groups = Vec::new();
        for label in &game.labels {
            if label.states.len() > 0 {
                groups.push(StateGroup {
                    name: label.name.clone(),
                    members: label.states.clone(),
                });
            }
        }

        Self::Grouped { groups }
    }

    pub fn len(&self) -> usize {
        match self {
            Self::Individual { state_indices } => state_indices.len(),
            Self::Grouped { groups } => groups.len(),
        }
    }

    pub fn remove_from_coalition(&self, game: &mut Game, index: usize) {
        self.add_or_remove_to_coalition(game, index, false);
    }

    pub fn add_to_coalition(&self, game: &mut Game, index: usize) {
        self.add_or_remove_to_coalition(game, index, true);
    }
    fn add_or_remove_to_coalition(&self, game: &mut Game, index: usize, add: bool) {
        match self {
            StateGroups::Individual { state_indices } => {
                game.add_or_remove_to_coalition(state_indices[index], add);
            }
            StateGroups::Grouped { groups } => {
                for &member in &groups[index].members {
                    game.add_or_remove_to_coalition(member, add);
                }
            }
        }
    }

    pub fn set_state_mask(&self, game: &mut Game, state_mask: u64) {
        self.set_or_clear_state_mask(game, state_mask, true);
    }

    pub fn clear_state_mask(&self, game: &mut Game, state_mask: u64) {
        self.set_or_clear_state_mask(game, state_mask, false);
    }

    fn set_or_clear_state_mask(&self, game: &mut Game, state_mask: u64, set: bool) {
        match self {
            Self::Individual { state_indices } => {
                for (i, &state_index) in state_indices.iter().enumerate() {
                    if state_mask & 1 << i != 0 {
                        game.add_or_remove_to_coalition(state_index, set);
                    }
                }
            }
            Self::Grouped { groups } => {
                for (i, group) in groups.iter().enumerate() {
                    if state_mask & 1 << i != 0 {
                        for &member in &group.members {
                            game.add_or_remove_to_coalition(member, set);
                        }
                    }
                }
            }
        }
    }

    pub fn get_name(&self, index: usize, transition_system: &TransitionSystem) -> String {
        match self {
            StateGroups::Individual { state_indices } => transition_system.states
                [state_indices[index]]
                .to_string(&transition_system.variables)
                .clone(),
            StateGroups::Grouped { groups } => groups[index].name.clone(),
        }
    }
}

pub struct CachedGameSolver<'a> {
    game: Game,
    pub state_groups: &'a StateGroups,
    minimal_coalitions: Vec<u64>,
    thread_count: usize,
    step: usize,
    silent: bool,
}

impl<'a> CachedGameSolver<'a> {
    pub fn new(
        game: Game,
        thread_count: usize,
        state_groups: &'a StateGroups,
    ) -> CachedGameSolver<'a> {
        Self {
            game,
            state_groups,
            minimal_coalitions: Vec::new(),
            thread_count,
            step: 4096,
            silent: false,
        }
    }

    pub fn set_silent(&mut self, silent: bool) {
        self.silent = silent;
    }

    pub fn prepare(&mut self) {
        let n = self.state_groups.len();
        if n > 64 {
            // Even computing it for this size is already intractable in practice, but at > 64
            // states, we hit a hard limit as coalitions are stored in an u64
            panic!("Responsibility can only be computed for games with up to 64 state (groups). Game has {} state (groups)", n);
        }
        let start_solve = std::time::Instant::now();

        let mut progress_reporter = if self.silent {
            None
        } else {
            Some(ProgressReporter::new(n as u32))
        };
        for size in 0..=n as u32 {
            if !self.silent {
                progress_reporter.as_mut().unwrap().set_size(size);
            }
            self.compute_minimal_coalitions_of_size(size);
        }

        if !self.silent {
            println!(
                "Found {} minimum coalitions in {:.2?}.",
                self.minimal_coalitions.len(),
                start_solve.elapsed()
            );
            println!();
        }
    }

    fn compute_minimal_coalitions_of_size(&mut self, size: u32) {
        let mut thread_states = Vec::with_capacity(self.thread_count);
        for _ in 0..self.thread_count {
            thread_states.push(SolverThreadState::new(self.game.clone()));
        }

        let coalition_count = 1u64 << self.state_groups.len();
        let coalitions = std::sync::Mutex::new((0..coalition_count).step_by(self.step));
        let step_is_power_of_two = self.step.is_power_of_two();
        if size == 0 && !step_is_power_of_two {
            println!(
                "Step size ({}) is not a power of two. Use a power of two for best performance",
                self.step
            );
        }

        thread_states.par_iter_mut().for_each(|thread_state| loop {
            let start = match coalitions.lock().unwrap().next() {
                None => break,
                Some(first_coalition) => first_coalition,
            };
            let start_ones = start.count_ones();
            if !(step_is_power_of_two && start_ones > size || start_ones + self.step.ilog2() < size)
            {
                for coalition in start..(start + self.step as u64).min(coalition_count) {
                    if coalition.count_ones() == size {
                        // This only checks against the minimal coalitions that have been found so far:
                        if !self.is_game_winning(coalition) {
                            self.solve_game(thread_state, coalition);
                        }
                    }
                }
            }
        });

        for thread_state in &mut thread_states {
            self.minimal_coalitions
                .append(&mut thread_state.new_minimal_coalitions);
        }
    }

    fn solve_game(&self, thread_state: &mut SolverThreadState, coalition: u64) {
        self.state_groups
            .set_state_mask(&mut thread_state.game, coalition);
        if thread_state.game.determine_winner() == Player::Safe {
            thread_state.new_minimal_coalitions.push(coalition);
        }
        self.state_groups
            .clear_state_mask(&mut thread_state.game, coalition);
    }

    pub fn is_game_winning(&self, coalition: u64) -> bool {
        for &minimal_coalition in &self.minimal_coalitions {
            if is_subset_of(minimal_coalition, coalition) {
                return true;
            }
        }
        return false;
    }
}

struct ProgressReporter {
    size_bar: ProgressBar,
}

impl ProgressReporter {
    pub fn new(max_size: u32) -> Self {
        let size_bar = ProgressBar::new(max_size as u64);
        size_bar.set_style(
            indicatif::ProgressStyle::with_template("Coalition size: {pos}/{len} [{wide_bar}]")
                .unwrap()
                .progress_chars("=> "),
        );
        Self { size_bar }
    }

    pub fn set_size(&mut self, size: u32) {
        self.size_bar.set_position(size as u64);
    }
}
