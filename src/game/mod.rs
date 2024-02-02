mod super_attractor;

use crate::transition_systems::TransitionSystem;
use std::collections::HashMap;

#[derive(Clone)]
pub struct Game {
    pub initial_state: usize,
    pub states: Vec<State>,
    pub state_predecessors: Vec<StatePredecessors>,
    pub bad_states: Vec<usize>,
    pub labels: Vec<Label>,
}

impl Default for Game {
    fn default() -> Self {
        Self {
            initial_state: 0,
            states: Vec::new(),
            state_predecessors: Vec::new(),
            bad_states: Vec::new(),
            labels: Vec::new(),
        }
    }
}

impl Game {
    pub fn from_transition_system(transition_system: &TransitionSystem) -> Self {
        let mut game = Game::default();

        let mut ts_index_to_game_index = HashMap::new();

        for (index, label) in &transition_system.label_names {
            ts_index_to_game_index.insert(index, game.labels.len());
            game.labels.push(Label::new(label.clone()));
        }
        let mut unlabelled_states = Label::new("unlabelled".to_string());

        for (i, state) in transition_system.states.iter().enumerate() {
            game.states.push(State::new(Player::Reach));
            game.state_predecessors.push(StatePredecessors::default());
            if state.is_bad {
                game.bad_states.push(game.states.len() - 1);
            }
            for label in &state.labels {
                game.labels[ts_index_to_game_index[label]].states.push(i)
            }
            if state.labels.is_empty() {
                unlabelled_states.states.push(i);
            }
        }

        if unlabelled_states.states.len() > 0 {
            game.labels.push(unlabelled_states);
        }

        for (source_index, source_state) in transition_system.states.iter().enumerate() {
            for transition in &source_state.outgoing_transitions {
                game.add_transition(source_index, transition.destination);
            }
        }

        game.initial_state = transition_system.initial_state;

        game
    }

    fn add_transition(&mut self, from: usize, to: usize) {
        self.states[from].successor_count += 1;
        self.state_predecessors[to]
            .predecessors
            .push(Transition::new(from));
    }

    pub fn mark_counterexample_path(&mut self, counterexample: Vec<usize>) {
        for &state in &counterexample {
            self.states[state].owner = Player::Path;
            self.states[state].default_owner = Player::Path;
        }

        for (&from, &to) in counterexample.iter().zip(counterexample.iter().skip(1)) {
            let mut marked_transitions = 0;
            for to_transition in &mut self.state_predecessors[to].predecessors {
                if to_transition.source == from {
                    to_transition.on_path = true;
                    marked_transitions += 1;
                }
            }
            if marked_transitions != 1 {
                panic!(
                    "{} transitions match the transition from state {} to state {}",
                    marked_transitions, from, to
                );
            }
        }
    }

    pub fn get_significant_states(&self) -> Vec<usize> {
        let mut res = Vec::new();
        for (state_index, state) in self.states.iter().enumerate() {
            if state.successor_count > 1 && !self.bad_states.contains(&state_index) {
                res.push(state_index);
            }
        }

        res
    }

    pub fn add_to_coalition(&mut self, state: usize) {
        self.states[state].owner = Player::Safe;
        self.states[state].change_count += 1;
    }

    pub fn remove_from_coalition(&mut self, state: usize) {
        self.states[state].change_count -= 1;
        if self.states[state].change_count == 0 {
            self.states[state].owner = self.states[state].default_owner;
        }
    }

    pub fn add_or_remove_to_coalition(&mut self, state: usize, add: bool) {
        if add {
            self.add_to_coalition(state);
        } else {
            self.remove_from_coalition(state);
        }
    }

    #[allow(dead_code)]
    pub fn set_coalition(&mut self, coalition: &[usize]) {
        for &state in coalition {
            self.states[state].owner = Player::Safe;
        }
    }

    #[allow(dead_code)]
    pub fn reset_coalition(&mut self, coalition: &[usize]) {
        for &state in coalition {
            self.states[state].owner = self.states[state].default_owner;
        }
    }

    pub fn determine_winner(&mut self) -> Player {
        self.reset_attractor_counts();

        let mut open_set = Vec::new();
        for bad_state in &self.bad_states {
            self.states[*bad_state].attractor_count = 0;
            open_set.push(*bad_state);
        }

        while let Some(open_index) = open_set.pop() {
            for transition in &self.state_predecessors[open_index].predecessors {
                let source_state = &mut self.states[transition.source];
                if source_state.attractor_count > 0 {
                    match source_state.owner {
                        Player::Reach => {
                            source_state.attractor_count -= 1;
                        }
                        Player::Safe => {
                            source_state.attractor_count -= 1;
                        }
                        Player::Path => {
                            if transition.on_path {
                                source_state.attractor_count -= 1;
                            }
                        }
                    }
                    if source_state.attractor_count == 0 {
                        if transition.source == self.initial_state {
                            return Player::Reach;
                        }
                        open_set.push(transition.source);
                    }
                }
            }
        }

        Player::Safe
    }

    fn reset_attractor_counts(&mut self) {
        for state in &mut self.states {
            state.attractor_count = if state.owner == Player::Safe {
                state.successor_count
            } else {
                1
            };
        }
    }

    #[allow(dead_code)]
    pub fn find_minimal_causes(&self) -> Vec<Vec<usize>> {
        let mut super_attractor = super_attractor::SuperAttractor::new(self.clone());
        super_attractor.run()
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Player {
    Reach,
    Safe,
    Path,
}

#[derive(Clone)]
pub struct State {
    pub successor_count: usize,
    pub owner: Player,
    pub default_owner: Player,
    pub attractor_count: usize,
    pub change_count: usize, // If multiple groups set the owner of this state to safe, this ensures the owner is only reset after all these groups reset the owner.
}

impl State {
    pub fn new(owner: Player) -> Self {
        Self {
            successor_count: 0,
            owner,
            default_owner: owner,
            attractor_count: 0,
            change_count: 0,
        }
    }
}

#[derive(Clone)]
pub struct StatePredecessors {
    pub predecessors: Vec<Transition>,
}

impl Default for StatePredecessors {
    fn default() -> Self {
        Self {
            predecessors: Vec::new(),
        }
    }
}

#[derive(Copy, Clone)]
pub struct Transition {
    source: usize,
    on_path: bool,
}

impl Transition {
    pub fn new(source: usize) -> Self {
        Self {
            source,
            on_path: false,
        }
    }
}

#[derive(Clone)]
pub struct Label {
    pub name: String,
    pub states: Vec<usize>,
}
impl Label {
    pub fn new(name: String) -> Self {
        Self {
            name,
            states: Vec::new(),
        }
    }
}
