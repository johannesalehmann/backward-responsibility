use crate::transition_systems::{State, Transition, TransitionSystem, Variable};
use std::collections::HashMap;
use std::fmt::Display;
use std::fs;

pub struct TransitionSystemParser {
    states_file_name: String,
    transitions_file_name: String,
    labels_file_name: String,
}

impl TransitionSystemParser {
    pub fn from_stem(results_file_stem: String) -> Self {
        let states_file_name = format!("{}.sta", results_file_stem);
        let transitions_file_name = format!("{}.tra", results_file_stem);
        let labels_file_name = format!("{}.lab", results_file_stem);
        Self {
            states_file_name,
            transitions_file_name,
            labels_file_name,
        }
    }
    pub fn from_files(
        states_file_name: String,
        transitions_file_name: String,
        labels_file_name: String,
    ) -> Self {
        Self {
            states_file_name,
            transitions_file_name,
            labels_file_name,
        }
    }

    pub fn parse(&self, bad_label: &str) -> TransitionSystem {
        let states_file = Self::get_file_content(&self.states_file_name, "model state file");
        let transitions_file =
            Self::get_file_content(&self.transitions_file_name, "model transition file");
        let labels_file = Self::get_file_content(&self.labels_file_name, "model label file");

        let (mut states, variables) = Self::parse_states_and_vars(states_file);
        Self::parse_transitions(transitions_file, &mut states);
        let (initial_state, label_names) = Self::parse_labels(labels_file, &mut states, bad_label);

        TransitionSystem::new(states, initial_state, variables, label_names)
    }

    pub fn get_file_content<P: AsRef<std::path::Path> + Display>(
        path: &P,
        description: &str,
    ) -> String {
        fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("Unable to open {} \"{}\": {}", description, path, e))
    }

    fn parse_states_and_vars(state_file: String) -> (Vec<State>, Vec<Variable>) {
        let mut lines = state_file.lines();
        let header = lines
            .next()
            .expect("State file must contain at least one line");

        let mut vars = Vec::new();
        for variable_name in header[1..header.len() - 1].split(",") {
            vars.push(Variable::new(variable_name.to_string()));
        }

        let mut states = Vec::new();
        for (i, state_line) in lines.enumerate() {
            let valuation_string = Self::extract_state_valuation_string(i, state_line);
            let valuation_indices = Self::parse_state_valuation_string(&mut vars, valuation_string);
            states.push(State::new(valuation_indices));
        }

        (states, vars)
    }

    fn extract_state_valuation_string(i: usize, state_line: &str) -> &str {
        let (index_string, valuations) = state_line.split_once(":").unwrap_or_else(|| {
            panic!(
                "Line {} of model state file does not contain expected colon",
                i + 1
            )
        });
        let parsed_index = index_string
            .parse::<usize>()
            .unwrap_or_else(|e| panic!("Failed to parse state index in model state file: {}", e));
        if parsed_index != i {
            panic!("States in model state file do not have consecutive indices: Expected {}, found {}.", i, parsed_index);
        }
        &valuations[1..valuations.len() - 1]
    }

    fn parse_state_valuation_string(vars: &mut Vec<Variable>, valuations: &str) -> Vec<usize> {
        let mut valuation_indices = Vec::new();
        for (i, valuation) in valuations.split(",").enumerate() {
            valuation_indices.push(vars[i].get_valuation_index_or_add(valuation));
        }
        if valuation_indices.len() != vars.len() {
            panic!(
                "A state has an incorrect number of variable valuations ({} instead of {})",
                valuation_indices.len(),
                vars.len()
            );
        }
        valuation_indices
    }

    fn parse_transitions(transitions_file: String, states: &mut Vec<State>) {
        let mut lines = transitions_file.lines();

        // Currently, we just skip the header without parsing it
        lines.next().expect("Transition file is missing header");

        for transition_line in lines {
            let (source, destination) = Self::parse_source_and_destination(transition_line);

            if source >= states.len() {
                panic!(
                    "Transition source is invalid ({}, there are {} states)",
                    source,
                    states.len()
                );
            }
            if destination >= states.len() {
                panic!(
                    "Transition destination is invalid ({}, there are {} states)",
                    destination,
                    states.len()
                );
            }

            states[source]
                .outgoing_transitions
                .push(Transition::new(destination));
        }
    }

    fn parse_source_and_destination(transition_line: &str) -> (usize, usize) {
        let mut components = transition_line.split(" ");
        let error_message = "Transition file entry is missing field: At least 4 entries (source, destination, action, probability) are required";
        let source: usize = components
            .next()
            .expect(error_message)
            .parse()
            .unwrap_or_else(|e| panic!("Could not parse transition source: {}", e));

        // We are not interested in the index of the action, so we simply skip it:
        components.next();

        let destination: usize = components
            .next()
            .expect(error_message)
            .parse()
            .unwrap_or_else(|e| panic!("Could not parse transition source: {}", e));

        let probability: f32 = components
            .next()
            .expect(error_message)
            .parse()
            .unwrap_or_else(|e| panic!("Could not parse probability: {}", e));
        if probability != 1.0 {
            panic!("Encountered transition with probability {}, but only transitions with probability 1 are supported.", probability);
        }
        (source, destination)
    }

    fn parse_labels(
        label_file: String,
        states: &mut Vec<State>,
        bad_label: &str,
    ) -> (usize, Vec<(usize, String)>) {
        let init_label_name = "init";

        let mut lines = label_file.lines();
        let header = lines
            .next()
            .expect("Label file must contain at least one line");

        let mut labels = HashMap::new();
        let mut label_names = Vec::new();
        for label_string in header.split(" ") {
            let (index_string, name_with_quotes) = label_string
                .split_once("=")
                .expect("Label header has incorrect format");
            let index: usize = index_string
                .parse()
                .unwrap_or_else(|e| panic!("Could not parse index of label: {}", e));
            let name = name_with_quotes.trim_matches('\"').to_string();
            labels.insert(index, name.clone());
            label_names.push((index, name));
        }

        let mut initial_state = None;

        for label_entry in lines {
            let (state_string, label_index_strings) = label_entry
                .split_once(": ")
                .expect("Label file entry has incorrect format");
            let state: usize = state_string
                .parse()
                .unwrap_or_else(|e| panic!("Failed to parse index of state with label: {}", e));
            for label_index_string in label_index_strings.split(" ") {
                let label_index: usize = label_index_string
                    .parse()
                    .unwrap_or_else(|e| panic!("Failed to parse label index: {}", e));

                if labels[&label_index] == init_label_name {
                    match initial_state {
                        Some(_) => panic!("Model must have exactly one initial state"),
                        None => initial_state = Some(state),
                    }
                } else if labels[&label_index] == bad_label {
                    states[state].is_bad = true;
                } else {
                    states[state].labels.push(label_index);
                }
            }
        }

        let initial_state = match initial_state {
            Some(state) => state,
            None => panic!("Model does not have initial state"),
        };

        (initial_state, label_names)
    }

    pub fn parse_counterexample_from_file<P: AsRef<std::path::Path> + Display>(
        file: P,
        ts: &TransitionSystem,
    ) -> Vec<usize> {
        let file = TransitionSystemParser::get_file_content(&file, "counterexample file");
        let lines = file.lines().map(|s| s.to_string()).collect::<Vec<_>>();
        if file.contains("=") {
            Self::parse_counterexample_with_varnames(lines, ts)
        } else {
            Self::parse_counterexample(lines, ts)
        }
    }

    pub fn parse_counterexample_with_varnames(
        ce_strings: Vec<String>,
        ts: &TransitionSystem,
    ) -> Vec<usize> {
        let mut ce = Vec::new();

        for ce_string in &ce_strings {
            let ce_string = &ce_string.trim()[1..ce_string.len() - 1];
            let mut val_indices = vec![None; ts.variables.len()];

            for assignment in ce_string.split(",") {
                let (name, value) = assignment.split_once("=").unwrap_or_else(|| {
                    panic!(
                        "Variable assignment \"{}\" has incorrect format, as it is missing \"=\".",
                        assignment
                    )
                });
                let name = name.trim();
                let value = value.trim();
                let var_index = ts
                    .get_variable_index(name)
                    .unwrap_or_else(|| panic!("Could not find variable \"{}\"", name));
                let val_index = ts.variables[var_index]
                    .get_valuation_index(value)
                    .unwrap_or_else(|| {
                        panic!(
                            "Could not find valuation \"{}\" for variable \"{}\".",
                            value, name
                        )
                    });
                if val_indices[var_index].is_some() {
                    panic!(
                        "Valuation contains two values for variable \"{}\".",
                        ts.variables[var_index].name
                    );
                }
                val_indices[var_index] = Some(val_index);
            }

            let valuation_indices = val_indices
                .into_iter()
                .enumerate()
                .map(|(i, v)| {
                    v.unwrap_or_else(|| {
                        panic!("Variable {} was not assigned a value", ts.variables[i].name)
                    })
                })
                .collect::<Vec<_>>();

            ce.push(
                ts.find_state_with_valuation(valuation_indices)
                    .unwrap_or_else(|| panic!("Could not find valuation ({}).", ce_string)),
            );
        }

        ce
    }

    pub fn parse_counterexample(ce_strings: Vec<String>, ts: &TransitionSystem) -> Vec<usize> {
        let mut ce = Vec::new();

        for ce_string in &ce_strings {
            let ce_string = &ce_string.trim()[1..ce_string.len() - 1];
            let mut val_indices = Vec::new();
            for (i, val) in ce_string.split(",").enumerate() {
                let val = val.trim();
                let val_index = ts.variables[i].get_valuation_index(val);
                val_indices.push(val_index.unwrap_or_else(|| {
                    panic!(
                        "Unknown value {} for variable {}",
                        val, ts.variables[i].name
                    )
                }));
            }
            if val_indices.len() != ts.variables.len() {
                panic!(
                    "A state in the counterexample has an incorrect number of variable valuations ({} instead of {})",
                    val_indices.len(),
                    ts.variables.len()
                );
            }

            let state_index = ts.find_state_with_valuation(val_indices);
            ce.push(state_index.expect(
                "Transition system does not contain one of the states contained in the counterexample",
            ))
        }

        ce
    }
}
