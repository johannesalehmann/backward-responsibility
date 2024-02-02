use crate::game::{Game, Player};
use std::collections::HashSet;
use std::fmt::{Formatter, Write};

#[allow(dead_code)]
pub struct SuperAttractor {
    game: Game,
}

impl SuperAttractor {
    pub fn new(game: Game) -> Self {
        Self { game }
    }

    pub fn run(&mut self) -> Vec<Vec<usize>> {
        for state in &mut self.game.states {
            // state.default_owner = Player::Safe;
            state.owner = Player::Safe;
        }

        let mut current_size_open = Vec::new();
        let bad_set = StateSet::from_states(self.game.bad_states.clone());
        current_size_open.push(CWSet::new(StateSet::empty(), self.attract(&bad_set)));
        let mut next_size_open = Vec::new();
        let mut past_sizes_open = Vec::new();

        let mut minimal_causes = Vec::new();

        let mut examined_counter = 0;
        let mut added_counter = 0;

        while !current_size_open.is_empty() || !next_size_open.is_empty() {
            let next_set = if let Some(next_set) = current_size_open.pop() {
                // if next_set.controlled.states.len() > 4 {
                //     panic!("too long, aborting");
                // }
                next_set
            } else {
                println!(
                    "Continuing to next size, examined {} sets in last phase, added {} to current",
                    examined_counter, added_counter
                );
                examined_counter = 0;
                added_counter = 0;
                std::mem::swap(&mut current_size_open, &mut next_size_open);
                continue;
            };

            examined_counter += 1;
            // println!(
            //     "  Examining ({:?}, {:?})",
            //     next_set.controlled, next_set.winning
            // );

            for &new_state in self.can_reach(&next_set.winning).states.iter() {
                let new_controlled = next_set.controlled.clone().with_state(new_state);
                if current_size_open
                    .iter()
                    .any(|cw| cw.controlled == new_controlled)
                {
                    continue;
                }
                let new_winning = self.attract(&next_set.winning.clone().with_state(new_state));
                if new_winning.contains(self.game.initial_state) {
                    if !minimal_causes.iter().any(|m| *m == new_winning) {
                        minimal_causes.push(new_winning);
                        // println!("    Found minimal cause: {:?}", new_controlled);
                    }
                } else {
                    let new_set = CWSet::new(new_controlled, new_winning);
                    // println!("    Adding set: {:?}", new_set);
                    added_counter += 1;
                    next_size_open.push(new_set);
                }
            }

            past_sizes_open.push(next_set);
        }

        minimal_causes
            .into_iter()
            .map(|m| m.states.into_iter().collect::<Vec<_>>())
            .collect()
    }

    fn attract(&mut self, winning_set: &StateSet) -> StateSet {
        self.game.reset_attractor_counts();
        for &state in &winning_set.states {
            self.game.states[state].attractor_count = 0;
        }
        let mut open = winning_set.clone().get_state_vector();
        let mut closed = Vec::new();
        while let Some(state) = open.pop() {
            for predecessor in &self.game.state_predecessors[state].predecessors {
                if self.game.states[predecessor.source].attractor_count > 0 {
                    self.game.states[predecessor.source].attractor_count -= 1;
                    if self.game.states[predecessor.source].attractor_count == 0 {
                        open.push(predecessor.source);
                    }
                }
            }
            closed.push(state);
        }

        StateSet::from_states(closed)
    }

    fn can_reach(&mut self, winning_set: &StateSet) -> StateSet {
        self.game.reset_attractor_counts();
        for &state in &winning_set.states {
            self.game.states[state].attractor_count = 0;
        }

        let mut can_reach = Vec::new();

        for &state in &winning_set.states {
            for predecessor in &self.game.state_predecessors[state].predecessors {
                if self.game.states[predecessor.source].attractor_count > 0 {
                    if self.game.states[predecessor.source].default_owner != Player::Path
                        || predecessor.on_path
                    {
                        can_reach.push(predecessor.source);
                    }
                }
            }
        }

        StateSet::from_states(can_reach)
    }
}

pub struct CWSet {
    controlled: StateSet,
    winning: StateSet,
}

impl CWSet {
    pub fn new(controlled: StateSet, winning: StateSet) -> Self {
        Self {
            controlled,
            winning,
        }
    }
}

impl std::fmt::Debug for CWSet {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_char('(')?;
        self.controlled.fmt(f)?;
        f.write_str(", ")?;
        self.winning.fmt(f)?;
        f.write_char(')')?;
        Ok(())
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct StateSet {
    states: HashSet<usize>,
}

impl StateSet {
    pub fn empty() -> Self {
        Self {
            states: HashSet::new(),
        }
    }

    pub fn from_states(states: Vec<usize>) -> Self {
        Self {
            states: HashSet::from_iter(states.into_iter()),
        }
    }

    pub fn with_state(mut self, state: usize) -> Self {
        self.states.insert(state);
        self
    }

    pub fn get_state_vector(self) -> Vec<usize> {
        self.states.into_iter().collect()
    }

    pub fn contains(&self, state: usize) -> bool {
        self.states.contains(&state)
    }

    #[allow(dead_code)]
    pub fn subset_of(&self, other: &StateSet) -> bool {
        for &this_state in &self.states {
            if !other.contains(this_state) {
                return false;
            }
        }
        true
    }
}

impl std::fmt::Debug for StateSet {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_char('{')?;
        f.write_str(
            &self
                .states
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
                .join(", "),
        )?;
        f.write_char('}')?;

        Ok(())
    }
}
