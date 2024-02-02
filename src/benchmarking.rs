use crate::cli::ResponsibilityVersion;
use crate::prism::transition_system_parser::TransitionSystemParser;
use crate::prism::{PrismInterface, PrismRunner, RunResults};
use crate::shapley::{SampleTarget, StateGroups, TrimmedResponsibilityResult, WeightType};
use comfy_table::{CellAlignment, Table};
use num_rational::BigRational;
use num_traits::{One, ToPrimitive};
use std::time::Duration;

pub struct Benchmarker {
    benchmarks: Vec<Benchmark>,
    grouped: bool,
    samples: usize,
    durations: Vec<f32>,
    prism_interface: PrismInterface,
    extra_time_per_run: f64,
    no_prism: bool,
}

impl Benchmarker {
    pub fn new(samples: usize, grouped: bool, prism_runner: PrismRunner, no_prism: bool) -> Self {
        let mut prism_interface = PrismInterface::new(prism_runner);
        prism_interface.set_silent(true);

        Self {
            benchmarks: Vec::new(),
            grouped,
            samples,
            durations: Vec::new(),
            prism_interface,
            extra_time_per_run: 1.0,
            no_prism,
        }
    }

    pub fn add_benchmark<S1: Into<String>, S2: Into<String>, S3: Into<String>>(
        &mut self,
        file: S1,
        latex_display_string: S2,
        sbar: S3,
        seed: u64,
        sample_counts: Option<Vec<usize>>,
    ) {
        self.benchmarks.push(Benchmark {
            file: file.into(),
            latex_display_string: latex_display_string.into(),
            sbar: sbar.into(),
            seed,
            sample_counts,
        })
    }

    pub fn add_duration(&mut self, duration: f32) {
        self.durations.push(duration);
    }

    pub fn get_reference_duration_index(&self) -> Option<usize> {
        self.durations
            .iter()
            .enumerate()
            .max_by(|(_, d1), (_, d2)| d1.total_cmp(d2))
            .map(|(i, _)| i)
    }

    pub fn estimate_duration(&self) -> f64 {
        let mut total_duration = 0.0;

        for &duration in &self.durations {
            total_duration += (duration as f64 + self.extra_time_per_run)
                * self.samples as f64
                * self.benchmarks.len() as f64;
        }

        total_duration
    }

    pub fn estimate_progress_before(
        &self,
        benchmark_index: usize,
        duration_index: usize,
        sample_index: usize,
    ) -> f64 {
        let mut accumulated_duration = 0.0;

        // Time for previous benchmarks:
        for &duration in &self.durations {
            accumulated_duration += (duration as f64 + self.extra_time_per_run)
                * self.samples as f64
                * benchmark_index as f64;
        }
        for &duration in &self.durations[0..duration_index] {
            accumulated_duration +=
                (duration as f64 + self.extra_time_per_run) * self.samples as f64;
        }
        accumulated_duration +=
            (self.durations[duration_index] as f64 + self.extra_time_per_run) * sample_index as f64;

        accumulated_duration
    }

    pub fn run(&mut self) {
        let mut results_table = Results::new(&self.durations[..]);
        let mut progress_bar = indicatif::ProgressBar::new(self.estimate_duration().ceil() as u64);
        progress_bar.set_style(
            indicatif::ProgressStyle::with_template("Coalition: {pos}s/{len}s [{wide_bar}] {msg}")
                .unwrap()
                .progress_chars("=> "),
        );

        let thread_count = rayon::current_num_threads();
        let reference_duration = self
            .get_reference_duration_index()
            .expect("You must benchmark with at least one duration");

        if !self.no_prism {
            self.prism_interface.verify();
        }

        for benchmark in &mut self.benchmarks {
            if benchmark.seed == 0 {
                benchmark.seed = fastrand::u64(..);
            }
        }

        for (i, benchmark) in self.benchmarks.iter().enumerate() {
            let (results, size, samples) =
                self.sample_benchmark(thread_count, i, benchmark, &mut progress_bar);
            results_table.start_new_row(
                benchmark.file.clone(),
                benchmark.latex_display_string.clone(),
                benchmark.sbar.clone(),
                size,
                benchmark.seed,
            );
            let state_count = results[0][0].0.len();

            let reference_means = (0..state_count)
                .map(|i| {
                    results[reference_duration]
                        .iter()
                        .map(|(s, _n)| s[i].total_value.to_f64().unwrap())
                        .sum::<f64>()
                        / results[reference_duration].len() as f64
                })
                .collect::<Vec<_>>();

            for i in 0..self.durations.len() {
                let mut difference_sum = 0.0;
                for (sample, n) in &results[i] {
                    results_table.add_sample_count(*n);

                    let mut squared_difference: f64 = 0.0;
                    for state in 0..state_count {
                        squared_difference += (&reference_means[state]
                            - sample[state].total_value.to_f64().unwrap())
                        .powi(2)
                    }
                    difference_sum += squared_difference.sqrt();
                }
                let standard_deviation = difference_sum / (self.samples as f64);

                let sufficient_coverage = results[i].iter().any(|(s, _n)| {
                    s.iter().map(|r| &r.total_value).sum::<BigRational>() >= BigRational::one()
                });
                results_table.add_entry(samples[i], standard_deviation, sufficient_coverage);
            }
            // self.print_tables(&results_table);
        }
        self.print_tables(&results_table);
    }

    fn print_tables(&self, results_table: &Results) {
        results_table.print();
        // println!();
        // println!("LaTeX version of table:");
        // results_table.print_latex();
        println!();
        println!("Benchmarking file for reproducibility:");
        results_table.print_benchmarking_file(self.samples, self.grouped);
    }

    fn get_ts_and_ce(&self, file: &str, sbar: &str) -> RunResults {
        if self.no_prism {
            let stem = file.trim_end_matches(".prism");

            let ts_parser = TransitionSystemParser::from_files(
                format!("{}.sta", stem),
                format!("{}.tra", stem),
                format!("{}.lab", stem),
            );
            let ts = ts_parser.parse(sbar);
            let ce =
                TransitionSystemParser::parse_counterexample_from_file(format!("{}.ce", stem), &ts);
            RunResults::new(ce, ts)
        } else {
            self.prism_interface
                .build_and_export(file, sbar.to_string())
        }
    }

    fn sample_benchmark(
        &self,
        thread_count: usize,
        benchmark_index: usize,
        benchmark: &Benchmark,
        progress_bar: &mut indicatif::ProgressBar,
    ) -> (
        Vec<Vec<(Vec<TrimmedResponsibilityResult>, usize)>>,
        usize,
        Vec<f64>,
    ) {
        fastrand::seed(benchmark.seed as u64);

        let mut results = Vec::new();
        let mut samples_by_duration = Vec::new();
        let mut size = 0;
        for (duration_index, &duration) in self.durations.iter().enumerate() {
            let mut samples = Vec::new();
            let mut sample_counts = Vec::new();
            for sample in 0..self.samples {
                progress_bar.set_position(self.estimate_progress_before(
                    benchmark_index,
                    duration_index,
                    sample,
                ) as u64);
                progress_bar.set_message(format!(
                    "{}, {}s: {}/{}",
                    &benchmark.file,
                    duration,
                    sample + 1,
                    self.samples
                ));

                let ts_ce = self.get_ts_and_ce(&benchmark.file, &benchmark.sbar);
                let mut game = crate::Game::from_transition_system(&ts_ce.transition_system);
                size = game.states.len();
                game.mark_counterexample_path(ts_ce.counterexample);
                let state_groups = if self.grouped {
                    StateGroups::grouped_by_label_from_game(&game)
                } else {
                    StateGroups::individual_from_game(&game)
                };
                let mut responsibility_calculator = crate::shapley::ResponsibilityCalculator::new(
                    game,
                    thread_count,
                    WeightType::Shapley,
                    state_groups,
                    ResponsibilityVersion::Pessimistic,
                );
                responsibility_calculator.set_silent(true);

                let target = match &benchmark.sample_counts {
                    Some(counts) => {
                        SampleTarget::Samples(counts[duration_index * self.samples + sample])
                    }
                    None => SampleTarget::ElapsedTime(Duration::from_secs_f32(duration)),
                };

                let resp = responsibility_calculator.sample_individual_responsibilities(target);
                samples.push((
                    resp.into_iter()
                        .map(|r| TrimmedResponsibilityResult::from_responsibility_result(r))
                        .collect::<Vec<_>>(),
                    responsibility_calculator.sampled_count,
                ));
                sample_counts.push(responsibility_calculator.sampled_count)
            }
            results.push(samples);
            samples_by_duration
                .push(sample_counts.iter().sum::<usize>() as f64 / sample_counts.len() as f64);
        }
        (results, size, samples_by_duration)
    }
}

pub struct Benchmark {
    file: String,
    latex_display_string: String,
    sbar: String,
    seed: u64,
    sample_counts: Option<Vec<usize>>,
}

pub struct Results {
    durations: Vec<f32>,
    rows: Vec<ResultsRow>,
}

impl Results {
    pub fn new(durations: &[f32]) -> Self {
        Self {
            durations: durations.to_vec(),
            rows: Vec::new(),
        }
    }

    pub fn start_new_row(
        &mut self,
        name: String,
        latex_display_string: String,
        bad_label: String,
        size: usize,
        seed: u64,
    ) {
        self.rows.push(ResultsRow {
            name,
            latex_display_string,
            bad_label,
            size,
            entries: Vec::new(),
            samples: Vec::new(),
            seed,
        })
    }

    pub fn add_entry(&mut self, samples: f64, standard_deviation: f64, any_above_one: bool) {
        self.rows.last_mut().unwrap().entries.push(ResultsEntry {
            samples,
            standard_deviation,
            any_above_one,
        });
    }

    pub fn add_sample_count(&mut self, count: usize) {
        self.rows.last_mut().unwrap().samples.push(count);
    }

    pub fn print_benchmarking_file(&self, sample_count: usize, grouped: bool) {
        println!("{}", sample_count);
        if grouped {
            println!("grouped");
        } else {
            println!("individual");
        }
        println!(
            "{}",
            self.durations
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
                .join(" ")
        );
        for row in &self.rows {
            println!(
                "{} {} {}:{} {}",
                row.name,
                row.bad_label,
                row.latex_display_string,
                row.seed,
                row.samples
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
                    .join(" ")
            );
        }
    }

    pub fn _print_latex(&self) {
        println!(
            "\\begin{{tabular}}{{l r |{}}}",
            " r r".repeat(self.durations.len())
        );
        print!("&");
        for duration in &self.durations {
            print!(" & \\multicolumn{{2}}{{c}}{{$t={}s$}}", duration);
        }
        println!(" \\\\");
        print!("name & states");
        for _ in &self.durations {
            print!(" & \\multicolumn{{1}}{{c}}{{$n$}} & \\multicolumn{{1}}{{c}}{{$\\sigma$}}");
        }
        println!(" \\\\");
        println!("\\hline");

        for (i, row) in self.rows.iter().enumerate() {
            print!("{} & ${}$", row.latex_display_string, row.size);
            for entry in &row.entries {
                print!(" & ",);
                if !entry.any_above_one {
                    print!("(")
                };
                print!("${:.1}$k", entry.samples / 1000.0,);
                if !entry.any_above_one {
                    print!(")")
                };
                print!(" & ");
                if !entry.any_above_one {
                    print!("(")
                };
                print!("${:.4}$", entry.standard_deviation);
                if !entry.any_above_one {
                    print!(")")
                };
            }
            if i != self.rows.len() - 1 {
                print!(" \\\\");
            }
            println!();
        }
        println!("\\end{{tabular}}");
    }

    pub fn print(&self) {
        let mut table = Table::new();
        table
            .load_preset(comfy_table::presets::UTF8_FULL)
            .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS);

        let mut header = Vec::new();
        header.push("Name".to_string());
        header.push("Size".to_string());
        for duration in &self.durations {
            header.push(format!("n (t={})", duration));
            header.push(format!("σ (t={})", duration));
        }
        table.set_header(header);

        for row in &self.rows {
            let mut row_data: Vec<String> = Vec::new();
            row_data.push(row.name.clone());
            row_data.push(row.size.to_string());
            for entry in &row.entries {
                row_data.push(format!("{:.0}", entry.samples));
                let (insig_start, insig_end) = match entry.any_above_one {
                    true => ("", ""),
                    false => ("(", ")"),
                };
                row_data.push(format!(
                    "{}{:.4}{}",
                    insig_start, entry.standard_deviation, insig_end
                ));
            }
            table.add_row(row_data);
        }
        table
            .column_mut(1)
            .unwrap()
            .set_cell_alignment(CellAlignment::Right);
        for i in 0..self.durations.len() {
            table
                .column_mut(2 * i + 2)
                .unwrap()
                .set_cell_alignment(CellAlignment::Right);
            table
                .column_mut(2 * i + 3)
                .unwrap()
                .set_cell_alignment(CellAlignment::Right);
        }
        println!("{}", table);
    }
}

pub struct ResultsRow {
    name: String,
    latex_display_string: String,
    bad_label: String,
    size: usize,
    entries: Vec<ResultsEntry>,
    seed: u64,
    samples: Vec<usize>,
}

pub struct ResultsEntry {
    samples: f64,
    standard_deviation: f64,
    any_above_one: bool,
}
