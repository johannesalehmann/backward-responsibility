mod runner;
pub mod transition_system_parser;

use crate::prism::transition_system_parser::TransitionSystemParser;
use crate::transition_systems::TransitionSystem;
pub use runner::PrismRunner;
use std::ffi::{OsStr, OsString};
use std::path::Path;

pub struct PrismInterface {
    runner: PrismRunner,
    silent: bool,
}

impl PrismInterface {
    pub fn new(runner: PrismRunner) -> Self {
        Self {
            runner,
            silent: false,
        }
    }

    pub fn set_silent(&mut self, silent: bool) {
        self.silent = silent;
    }

    pub fn verify(&self) {
        let version = self.runner.run_prism(["-version"]);
        println!(
            "Found the following prism instance: {}",
            version
                .strip_suffix("\n")
                .or(version.strip_suffix("\r\n"))
                .unwrap_or(&version)
        );
    }

    pub fn build_and_export<S: Into<OsString> + Clone>(
        &self,
        model_file_name: S,
        bad_label: String,
    ) -> RunResults {
        let property = format!("E [F \"{}\"];", bad_label.as_str());
        let output = self.run_prism_export(model_file_name, property);
        let counterexample_string = self.parse_counterexample_from_output(output.stdout);
        let ts_parser = TransitionSystemParser::from_stem(output.results_file_name_stem);
        let transition_system = ts_parser.parse(bad_label.as_str());
        let counterexample =
            TransitionSystemParser::parse_counterexample(counterexample_string, &transition_system);

        RunResults::new(counterexample, transition_system)
    }

    fn generate_results_file_name(model_file_name: &OsStr) -> String {
        let model_stem = Path::new(&model_file_name).file_stem();
        if let Some(model_stem) = model_stem {
            format!("{}_results", model_stem.to_string_lossy())
        } else {
            "results".to_string()
        }
    }

    fn run_prism_export<S: Into<OsString>>(
        &self,
        model_file_name: S,
        property: String,
    ) -> RunOutput {
        let model_file_name = model_file_name.into();
        let property = property.into();

        let results_file = Self::generate_results_file_name(&model_file_name);
        if !self.silent {
            println!("Storing results in \"{}.all\"", results_file);
        }

        let output = self.runner.run_prism([
            OsString::from("-exportmodel"),
            OsString::from(format!("{}.all", results_file)),
            model_file_name,
            OsString::from("-pf"),
            property,
        ]);
        // println!("Full output: {}", output);
        RunOutput {
            stdout: output,
            results_file_name_stem: results_file,
        }
    }

    fn parse_counterexample_from_output(&self, output: String) -> Vec<String> {
        let mut counterexample = Vec::new();
        enum ParserState {
            BeforeCounterexample,
            InCounterexample,
            AfterCounterexample,
        }
        let mut state = ParserState::BeforeCounterexample;
        for line in output.lines() {
            match state {
                ParserState::BeforeCounterexample => {
                    if line.starts_with("Counterexample/witness") {
                        state = ParserState::InCounterexample;
                    }
                }
                ParserState::InCounterexample => {
                    if line.starts_with("(") {
                        counterexample.push(line.to_string());
                    } else {
                        state = ParserState::AfterCounterexample
                    }
                }
                ParserState::AfterCounterexample => {}
            }
        }
        counterexample
    }
}

struct RunOutput {
    stdout: String,
    results_file_name_stem: String,
}

pub struct RunResults {
    pub counterexample: Vec<usize>,
    pub transition_system: TransitionSystem,
}

impl Default for RunResults {
    fn default() -> Self {
        Self {
            counterexample: Vec::new(),
            transition_system: TransitionSystem::default(),
        }
    }
}

impl RunResults {
    pub fn new(counterexample: Vec<usize>, transition_system: TransitionSystem) -> Self {
        Self {
            counterexample,
            transition_system,
        }
    }
}
