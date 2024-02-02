use crate::shapley::{SampleTarget, WeightType};
use clap::{Arg, ArgAction, Command, ValueHint};
use std::time::Duration;

pub struct Settings {
    pub prism_config: PrismConfig,
    pub thread_count: Option<usize>,
    pub responsibility_metric: WeightType,
    pub responsibility_version: ResponsibilityVersion,
    pub subcommand: Subcommand,
    pub no_prism: bool,
}

#[derive(Eq, PartialEq)]
pub enum ResponsibilityVersion {
    Optimistic,
    Pessimistic,
}

pub struct PrismConfig {
    pub path: String,
    pub java_path: Option<String>,
}

pub enum Subcommand {
    Run(RunSubcommand),
    Benchmark(BenchmarkSubcommand),
}

pub struct RunSubcommand {
    pub model_input: ModelInput,
    pub counterexample_input: CounterexampleInput,
    pub bad_label: Option<String>,
    pub responsibility_output: ResponsibilityOutput,
    pub engine: Engine,
    pub grouped: bool,
}

pub struct BenchmarkSubcommand {
    pub file: String,
}

pub enum ModelInput {
    PrismFile {
        file: String,
    },
    TransitionSystemFile {
        state_file: String,
        transition_file: String,
        label_file: String,
    },
}

impl ModelInput {
    pub fn apply_no_prism(
        self,
        no_prism: bool,
        counterexample: CounterexampleInput,
    ) -> (Self, CounterexampleInput) {
        if !no_prism {
            (self, counterexample)
        } else {
            let (model, stem) = match self {
                ModelInput::PrismFile { file } => {
                    let stem = file.trim_end_matches(".prism");
                    (
                        ModelInput::TransitionSystemFile {
                            state_file: format!("{}.sta", stem),
                            transition_file: format!("{}.tra", stem),
                            label_file: format!("{}.lab", stem),
                        },
                        Some(stem.to_string()),
                    )
                }
                ModelInput::TransitionSystemFile {
                    state_file,
                    transition_file,
                    label_file,
                } => (
                    ModelInput::TransitionSystemFile {
                        state_file,
                        transition_file,
                        label_file,
                    },
                    None,
                ),
            };
            let counterexample = match counterexample {
                CounterexampleInput::ModelChecker => CounterexampleInput::File {file: format!("{}.ce", stem.expect("Cannot use --no-prism when model input is already a preprocessed transition system and counterexample is not specified."))},
                CounterexampleInput::File { file } => CounterexampleInput::File { file },
            };
            (model, counterexample)
        }
    }
}

pub enum CounterexampleInput {
    ModelChecker,
    File { file: String },
}

pub enum ResponsibilityOutput {
    Stdout,
    File { file: String },
}

pub enum Engine {
    Exact,
    Stochastic(SampleTarget),
}

impl Engine {
    pub fn is_stochastic(&self) -> bool {
        match self {
            Self::Exact => false,
            Self::Stochastic(_) => true,
        }
    }
}

impl Settings {
    pub fn parse() -> Self {
        let command = Command::new("bw-responsibility")
            .arg_required_else_help(true)
            .about("Compute backwards responsibility in non-probabilistic PRISM models")
            .version("1.0")
            .arg(
                Arg::new("benchmark")
                    .long("benchmark")
                    .short('e')
                    .help("Benchmark all models in the specified file. The file must contain the number of samples in the first line, a space-separated list of durations (specified in seconds, without unit) in the second line, followed by one model and bad property (space-separated) per line. The output is then data on how accurately responsibility can be computed for each duration and model.")
                    .num_args(1)
                    .value_hint(ValueHint::FilePath)
            )
            .arg(
                Arg::new("model-input-file")
                    .long("prism-model")
                    .short('p')
                    .help("The prism input model")
                    .value_name("model.prism")
                    .required_unless_present_any(&["state-file", "benchmark"])
                    .conflicts_with("benchmark")
                    .value_hint(ValueHint::FilePath)
                    .num_args(1)
            )
            .arg(
                Arg::new("bad-label")
                    .long("bad-label")
                    .short('b')
                    .help("The label in the model that should be avoided")
                    .required_unless_present_any(&["benchmark"])
                    .conflicts_with("benchmark")
                    .value_hint(ValueHint::Other)
                    .num_args(1)
            )
            .arg(
                Arg::new("metric")
                    .long("metric")
                    .short('m')
                    .help("The metric for responsibility. Possible values: shapley, banzhaf, count")
                    .default_value("shapley")
                    .value_hint(ValueHint::Other)
                    .num_args(1)
            )
            .arg(
                Arg::new("grouped")
                    .long("grouped")
                    .short('g')
                    .conflicts_with("benchmark")
                    .action(ArgAction::SetTrue)
                    .help("If set, states are grouped by labels, i.e. either all states with a given label are in the coalition or none of them are. Note that labels may overlap and that states with no labels are also allowed.")
                    .num_args(0)
            )
            .arg(
                Arg::new("randomised")
                    .long("randomised")
                    .short('r')
                    .conflicts_with("benchmark")
                    .help("Use a randomised sampler instead of the exact engine. Argument must either be the number of samples or the sampling duration in seconds (i.e. '-r 10000' or '-r 60s').")
                    .num_args(1)
            )
            .arg(
                Arg::new("responsibility-version")
                    .long("responsibility-version")
                    .short('v')
                    .help("Determines whether optimistic or pessimistic responsibility is computed. \"-v o\" selects optimistic responsibility,  \"-v p\" selects pessimistic responsibility. By default, pessimistic responsibility is computed. ")
                    .default_value("p")
                    .num_args(1)
            )
            .arg(
                Arg::new("state-file")
                    .long("state-file")
                    .short('s')
                    .help("The state file describing the transition system. If enabled, PRISM is not called and the transition system is instead read directly.")
                    .requires("transition-file")
                    .requires("label-file")
                    .requires("counterexample")
                    .conflicts_with("model-input-file")
                    .conflicts_with("benchmark")
                    .value_name("state_file.sta")
                    .value_hint(ValueHint::FilePath)
                    .num_args(1),
            )
            .arg(
                Arg::new("transition-file")
                    .long("transition-file")
                    .short('t')
                    .help("The transition file describing the transition system. If enabled, PRISM is not called and the transition system is instead read directly.")
                    .requires("state-file")
                    .requires("label-file")
                    .requires("counterexample")
                    .conflicts_with("model-input-file")
                    .conflicts_with("benchmark")
                    .value_name("transition_file.tra")
                    .value_hint(ValueHint::FilePath)
                    .num_args(1),
            )
            .arg(
                Arg::new("label-file")
                    .long("label-file")
                    .short('l')
                    .help("The label file describing the transition system. If enabled, PRISM is not called and the transition system is instead read directly.")
                    .requires("state-file")
                    .requires("transition-file")
                    .requires("counterexample")
                    .conflicts_with("model-input-file")
                    .conflicts_with("benchmark")
                    .value_name("label_file.lab")
                    .value_hint(ValueHint::FilePath)
                    .num_args(1),
            )
            .arg(
                Arg::new("counterexample")
                    .long("counterexample")
                    .short('c')
                    .help("A file that contains a counterexample. The file must have the same format as PRISM's counterexamples with one state per line. If there are three variables with values 1, 3 and 12, then a state has form (1,3,12).")
                    .value_name("counterexample.ce")
                    .value_hint(ValueHint::FilePath)
                    .num_args(1)
                    .required_unless_present_any(&["model-input-file", "benchmark"])
                    .conflicts_with("benchmark")
            )
            .arg(
                Arg::new("thread-count")
                    .long("thread-count")
                    .short('j')
                    .help("The number of worker threads to be used. Defaults to the number of (logical) cores")
                    .value_hint(ValueHint::Other)
                    .num_args(1)
            )
            .arg(
                Arg::new("responsibility-file")
                    .long("responsibility-file")
                    .short('f')
                    .help("The file where the responsibility values should be stored. Set to stdout to print to stdout.")
                    .default_value("stdout")
                    .conflicts_with("benchmark")
                    .value_hint(ValueHint::FilePath)
                    .num_args(1)
            )
            .arg(
                Arg::new("no-prism")
                    .long("no-prism")
                    .short('n')
                    .help("Replaces all references to PRISM models with references to transition, state, label and counterexample files with the same name by changing file extensions. This only works if the preprocessed transition system and counterexample are available.")
                    .num_args(0),
            )
            .arg(
                Arg::new("prism-path")
                    .long("prism-path")
                    .help("Command that executes PRISM.")
                    .default_value("prism")
                    .value_name("location of prism executable")
                    .value_hint(ValueHint::CommandName)
                    .num_args(1),
            )
            .arg(
                Arg::new("prism-java")
                    .long("prism-java")
                    .help("The directory of the java installation that PRISM should use. Only required if Java is not found in the default location.")
                    .value_name("path to java home")
                    .value_hint(ValueHint::DirPath)
                    .num_args(1),
            );

        let matches = command.get_matches();

        let prism_config = PrismConfig {
            path: matches.get_one::<String>("prism-path").unwrap().to_string(),
            java_path: matches
                .get_one::<String>("prism-java")
                .map(|p| p.to_string()),
        };

        let subcommand = if let Some(benchmark_file) = matches.get_one::<String>("benchmark") {
            Subcommand::Benchmark(BenchmarkSubcommand {
                file: benchmark_file.clone(),
            })
        } else {
            let model_input = if let Some(state_file) = matches.get_one::<String>("state-file") {
                let transition_file = matches
                    .get_one::<String>("transition-file")
                    .unwrap()
                    .to_string();
                let label_file = matches.get_one::<String>("label-file").unwrap().to_string();
                ModelInput::TransitionSystemFile {
                    state_file: state_file.to_string(),
                    transition_file,
                    label_file,
                }
            } else {
                let prism_file = matches.get_one::<String>("model-input-file").unwrap();
                ModelInput::PrismFile {
                    file: prism_file.to_string(),
                }
            };

            let counterexample_input = matches.get_one::<String>("counterexample").map_or(
                CounterexampleInput::ModelChecker,
                |c| CounterexampleInput::File {
                    file: c.to_string(),
                },
            );

            let grouped = matches.get_flag("grouped");

            let responsibility_output = matches.get_one::<String>("responsibility-file").map_or(
                ResponsibilityOutput::Stdout,
                |f| {
                    if f == "stdout" {
                        ResponsibilityOutput::Stdout
                    } else {
                        ResponsibilityOutput::File {
                            file: f.to_string(),
                        }
                    }
                },
            );

            let bad_label = matches
                .get_one::<String>("bad-label")
                .map(|s| s.to_string());

            let engine = match matches.get_one::<String>("randomised") {
                Some(value) => {
                    let target = if value.ends_with("s") {
                        SampleTarget::ElapsedTime(Duration::from_secs_f64(
                            value[..value.len() - 1]
                                .parse()
                                .expect("Could not parse randomised target duration."),
                        ))
                    } else {
                        SampleTarget::Samples(
                            value
                                .parse::<usize>()
                                .expect("Could not parse randomised target sample count."),
                        )
                    };
                    Engine::Stochastic(target)
                }
                None => Engine::Exact,
            };

            Subcommand::Run(RunSubcommand {
                model_input,
                counterexample_input,
                bad_label,
                responsibility_output,
                engine,
                grouped,
            })
        };

        let responsibility_metric = match matches.get_one::<String>("metric").unwrap().as_str() {
            "shapley" => WeightType::Shapley,
            "banzhaf" => WeightType::Banzhaf,
            "count" => WeightType::Count,
            metric => panic!("Unknown metric {}", metric),
        };
        let thread_count = matches
            .get_one::<String>("thread-count")
            .map(|t| t.parse::<usize>().expect("Could not parse thread-count."));

        let responsibility_version = match matches
            .get_one::<String>("responsibility-version")
            .unwrap()
            .as_str()
        {
            "o" => ResponsibilityVersion::Optimistic,
            "p" => ResponsibilityVersion::Pessimistic,
            version => panic!("Unknown responsibility version {}", version),
        };

        let no_prism = matches.get_flag("no-prism");

        Settings {
            prism_config,
            responsibility_version,
            responsibility_metric,
            thread_count,
            subcommand,
            no_prism,
        }
    }
}
