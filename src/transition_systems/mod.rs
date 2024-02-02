pub struct TransitionSystem {
    pub states: Vec<State>,
    pub initial_state: usize,
    pub variables: Vec<Variable>,
    pub label_names: Vec<(usize, String)>,
}

impl TransitionSystem {
    pub fn new(
        states: Vec<State>,
        initial_state: usize,
        variables: Vec<Variable>,
        label_names: Vec<(usize, String)>,
    ) -> Self {
        Self {
            states,
            initial_state,
            variables,
            label_names,
        }
    }

    pub fn get_variable_index(&self, name: &str) -> Option<usize> {
        for (i, variable) in self.variables.iter().enumerate() {
            if variable.name == name {
                return Some(i);
            }
        }
        None
    }

    pub fn find_state_with_valuation(&self, valuation: Vec<usize>) -> Option<usize> {
        for (index, state) in self.states.iter().enumerate() {
            let mut matches = true;
            if state.valuation_indices.len() != valuation.len() {
                matches = false;
            }
            for (state_val, other_val) in state.valuation_indices.iter().zip(valuation.iter()) {
                if *state_val != *other_val {
                    matches = false;
                }
            }
            if matches {
                return Some(index);
            }
        }

        None
    }

    pub fn verify_counterexample(&self, counterexample: &Vec<usize>) {
        if counterexample.len() == 0 {
            panic!("Counterexample must not be empty");
        }
        if counterexample[0] != self.initial_state {
            panic!("Counterexample does not start in initial state");
        }
        for (&from, &to) in counterexample.iter().zip(counterexample.iter().skip(1)) {
            if !self.states[from].has_transition_to(to) {
                panic!(
                    "Counterexample transition from ({}) to ({}) does not exist.",
                    self.states[from].to_string(&self.variables),
                    self.states[to].to_string(&self.variables)
                )
            }
        }
        if !self.states[counterexample[counterexample.len() - 1]].is_bad {
            panic!("Last state of counterexample is not labelled as bad state.")
        }
    }
}

impl Default for TransitionSystem {
    fn default() -> Self {
        Self {
            states: Vec::new(),
            initial_state: 0,
            variables: Vec::new(),
            label_names: Vec::new(),
        }
    }
}

pub struct State {
    pub outgoing_transitions: Vec<Transition>,
    pub valuation_indices: Vec<usize>,
    pub is_bad: bool,
    pub labels: Vec<usize>,
}

impl State {
    pub fn new(valuation_indices: Vec<usize>) -> Self {
        Self {
            valuation_indices,
            outgoing_transitions: Vec::new(),
            is_bad: false,
            labels: Vec::new(),
        }
    }

    pub fn to_string(&self, variables: &Vec<Variable>) -> String {
        self.valuation_indices
            .iter()
            .zip(variables.iter())
            .map(|(&val, var)| format!("{}={}", var.name, var.valuation_names[val]))
            .collect::<Vec<String>>()
            .join(", ")
    }

    pub fn has_transition_to(&self, to: usize) -> bool {
        self.outgoing_transitions
            .iter()
            .any(|t| t.destination == to)
    }
}

pub struct Transition {
    pub destination: usize,
}

impl Transition {
    pub fn new(destination: usize) -> Self {
        Self { destination }
    }
}

pub struct Variable {
    pub name: String,
    valuation_names: Vec<String>,
}
impl Variable {
    pub fn new(name: String) -> Self {
        Self {
            name,
            valuation_names: Vec::new(),
        }
    }

    pub fn get_valuation_index_or_add(&mut self, valuation: &str) -> usize {
        for (i, v) in self.valuation_names.iter().enumerate() {
            if v == valuation {
                return i;
            }
        }
        self.valuation_names.push(valuation.to_string());
        self.valuation_names.len() - 1
    }

    pub fn get_valuation_index(&self, valuation: &str) -> Option<usize> {
        for (i, v) in self.valuation_names.iter().enumerate() {
            if v == valuation {
                return Some(i);
            }
        }
        None
    }
}
