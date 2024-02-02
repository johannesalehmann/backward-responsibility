use crate::cli::{
    CounterexampleInput, Engine, ModelInput, ResponsibilityOutput, ResponsibilityVersion,
    Subcommand,
};
use crate::game::Game;
use crate::prism::transition_system_parser::TransitionSystemParser;
use crate::shapley::{ResponsibilityCalculator, ResponsibilityResult, StateGroups, WeightType};
use crate::transition_systems::TransitionSystem;
use num_rational::BigRational;
use num_traits::{One, Signed, ToPrimitive};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

mod benchmarking;
mod cli;
mod game;
mod prism;
mod shapley;
mod transition_systems;

fn main() {
    // std::panic::set_hook(Box::new(|error| {
    //     if let Some(message) = error.payload().downcast_ref::<&str>() {
    //         println!("ERROR: {}", message.red());
    //         println!("{}", "bw-responsibility did not run successfully.".red());
    //     } else {
    //         if let Some(location) = error.location() {
    //             println!(
    //                 "{}",
    //                 format!("bw-responsibility encountered a problem at {}", location).red()
    //             );
    //         } else {
    //             println!(
    //                 "{}",
    //                 format!("bw-responsibility encountered a problem at an unknown location.")
    //                     .red()
    //             );
    //         }
    //     }
    // }));

    let settings = cli::Settings::parse();

    let mut prism_runner = prism::PrismRunner::default();

    prism_runner.set_path_to_prism(settings.prism_config.path);
    if let Some(java_path) = settings.prism_config.java_path {
        prism_runner.set_path_to_java(java_path);
    }

    match settings.subcommand {
        Subcommand::Run(run_command) => {
            let bad_label = run_command.bad_label.expect(
                "--bad-label must be set as bad-label auto-detection is not yet supported.",
            );

            let (model_input, counterexample_input) = run_command
                .model_input
                .apply_no_prism(settings.no_prism, run_command.counterexample_input);

            let (ts, ce) = match model_input {
                ModelInput::PrismFile { file } => {
                    let prism_interface = prism::PrismInterface::new(prism_runner);
                    prism_interface.verify();
                    let result = prism_interface.build_and_export(file, bad_label);
                    (result.transition_system, Some(result.counterexample))
                }
                ModelInput::TransitionSystemFile {
                    state_file,
                    transition_file,
                    label_file,
                } => {
                    let ts_parser =
                        TransitionSystemParser::from_files(state_file, transition_file, label_file);
                    (ts_parser.parse(bad_label.as_str()), None)
                }
            };

            let ce = match counterexample_input {
                CounterexampleInput::ModelChecker => {
                    let ce = ce.expect(
                        "Counterexample must be provided externally in this configuration.",
                    );

                    print_counterexample(&ts, &ce);
                    ce
                }
                CounterexampleInput::File { file } => {
                    println!(
                        "Using counterexample from \"{}\" instead of PRISM output.",
                        file
                    );
                    TransitionSystemParser::parse_counterexample_from_file(&file, &ts)
                }
            };

            ts.verify_counterexample(&ce);

            let mut game = Game::from_transition_system(&ts);
            game.mark_counterexample_path(ce);

            let thread_count = match settings.thread_count {
                Some(thread_count) => thread_count,
                None => rayon::current_num_threads(),
            };

            let state_groups = if run_command.grouped {
                println!("Grouping states by label.");
                StateGroups::grouped_by_label_from_game(&game)
            } else {
                match settings.responsibility_version {
                    ResponsibilityVersion::Optimistic => StateGroups::individual_on_path(&game),
                    ResponsibilityVersion::Pessimistic => StateGroups::individual_from_game(&game),
                }
            };

            let mut responsibility_calculator = ResponsibilityCalculator::new(
                game,
                thread_count,
                settings.responsibility_metric,
                state_groups,
                settings.responsibility_version,
            );
            let mut responsibilities = match run_command.engine {
                Engine::Exact => responsibility_calculator.compute_individual_responsibility(),
                Engine::Stochastic(target) => {
                    println!("Using stochastic engine.");
                    responsibility_calculator.sample_individual_responsibilities(target)
                }
            };
            responsibilities.sort_by(|x, y| y.total_value.cmp(&x.total_value));
            match run_command.responsibility_output {
                ResponsibilityOutput::Stdout => {
                    print_responsibility(
                        &responsibilities,
                        &ts,
                        settings.responsibility_metric,
                        &responsibility_calculator.state_groups,
                        run_command.engine.is_stochastic(),
                    );
                }
                ResponsibilityOutput::File { file } => store_responsibility(
                    &responsibilities,
                    &ts,
                    settings.responsibility_metric,
                    &responsibility_calculator.state_groups,
                    file,
                ),
            };
        }
        Subcommand::Benchmark(benchmark_command) => {
            let file = std::fs::read_to_string(&benchmark_command.file)
                .unwrap_or_else(|e| panic!("Unable to open \"{}\": {}", benchmark_command.file, e));
            let benchmark_file_name = PathBuf::from(benchmark_command.file);
            let base_path = benchmark_file_name
                .parent()
                .expect("Could not determine base path.");
            println!("Base path: {:?}", base_path);

            let mut lines = file
                .lines()
                .filter(|l| !l.starts_with("//"))
                .map(|l| l.trim());
            let samples = lines
                .next()
                .expect("Benchmark file must give number of samples on second line")
                .parse::<usize>()
                .expect("Could not parse number of samples");
            let grouped = match lines
                .next()
                .expect("Benchmark must specify \"grouped\" or \"individual\" on third line")
            {
                "individual" => false,
                "grouped" => true,
                _ => {
                    panic!("Benchmark must specify \"grouped\" or \"individual\" on third line");
                }
            };

            let mut benchmarker =
                benchmarking::Benchmarker::new(samples, grouped, prism_runner, settings.no_prism);
            let duration_line = lines.next().expect(
                "Benchmark file must give a space-separated list of durations on the second line",
            );

            duration_line
                .split(' ')
                .map(|d| {
                    d.parse::<f32>()
                        .unwrap_or_else(|e| panic!("Could not parse direction \"{}\": {}", d, e))
                })
                .for_each(|d| benchmarker.add_duration(d));

            for line in lines {
                let (file, rest) = line.split_once(' ').unwrap_or_else(|| panic!("Each entry in the benchmark file must consist of a file name and a bad label, separated by a space. \"{}\" does not contain a space.", line));
                let (bad_label, latex_display_string, seed, sample_counts) = match rest
                    .split_once(' ')
                {
                    Some((bad_label, rest)) => match rest.split_once(':') {
                        Some((latex_display_string, rest)) => {
                            let (seed, samples) = rest.split_once(' ').unwrap();
                            let seed = seed
                                .parse::<u64>()
                                .expect("Could not parse benchmarking seed");
                            let samples = samples
                                .split(' ')
                                .map(|s| s.parse::<usize>().expect("Could not parse sample count"))
                                .collect::<Vec<_>>();
                            {
                                (
                                    bad_label,
                                    latex_display_string.to_string(),
                                    seed,
                                    Some(samples),
                                )
                            }
                        }
                        None => (bad_label, rest.to_string(), 0, None),
                    },
                    None => (
                        rest,
                        format!("\texttt{{{}}}", file.replace("_", "\\_")),
                        0,
                        None,
                    ),
                };

                let actual_path = base_path.join(std::path::Path::new(file));
                println!("Adding {:?}", actual_path);
                benchmarker.add_benchmark(
                    actual_path.to_str().unwrap().to_string(),
                    latex_display_string,
                    bad_label,
                    seed,
                    sample_counts,
                );
            }
            benchmarker.run();
            return;
        }
    }
}

fn print_counterexample(transition_system: &TransitionSystem, counterexample: &Vec<usize>) {
    println!("\nCounterexample:");
    for &state_index in counterexample {
        let state = &transition_system.states[state_index];
        println!("({})", state.to_string(&transition_system.variables));
    }
    println!();
}

fn print_responsibility(
    responsibilities: &Vec<ResponsibilityResult>,
    transition_system: &TransitionSystem,
    metric: WeightType,
    state_groups: &StateGroups,
    is_stochastic: bool,
) {
    println!("\nResponsibilities ({}):", metric);
    let mut sum = BigRational::new(0.into(), 1.into());
    for responsibility in responsibilities {
        sum += &responsibility.total_value;
        if responsibility.total_value.is_positive() {
            println!(
                "({}): {}",
                responsibility.group_index,
                responsibility.to_string(transition_system, state_groups)
            );
        }
    }

    if let Some(sum_f64) = sum.to_f64() {
        println!("Sum of responsibilities: {}", sum_f64);
        if !is_stochastic && metric == WeightType::Shapley && !sum.is_one() {
            println!("  Sum is not 1, but it should be for Shapley weights.");
            if (sum_f64 - 1.0).abs() < 0.00001 {
                println!("  Exact value: {}", sum);
            }
        }
    }
}

fn store_responsibility(
    responsibilities: &Vec<ResponsibilityResult>,
    transition_system: &TransitionSystem,
    metric: WeightType,
    state_groups: &StateGroups,
    file_name: String,
) {
    let mut file = File::create(&file_name)
        .unwrap_or_else(|e| panic!("Could not create responsibility output file: {}", e));
    writeln!(file, "Metric: {}", metric)
        .unwrap_or_else(|e| panic!("Could not write to responsibility output file: {}", e));
    for responsibility in responsibilities {
        writeln!(
            file,
            "{}",
            responsibility.to_string(transition_system, state_groups)
        )
        // file.write_all(responsibility.to_string(transition_system).as_bytes())
        .unwrap_or_else(|e| panic!("Could not write to responsibility output file: {}", e));
    }
    println!("Stored responsibility values in \"{}\"", file_name);
}
