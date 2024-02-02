use crate::game::{Game, Player};
use crate::transition_systems::TransitionSystem;
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::cast::ToPrimitive;
use num_traits::{One, Signed, Zero};
use rayon::prelude::*;
use std::fmt::{Display, Formatter};
use std::time::Duration;

mod game_solving;
use crate::cli::ResponsibilityVersion;
pub use game_solving::StateGroups;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum WeightType {
    Shapley,
    Banzhaf,
    Count,
}

impl Display for WeightType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            WeightType::Shapley => f.write_str("Shapley"),
            WeightType::Banzhaf => f.write_str("Banzhaf"),
            WeightType::Count => f.write_str("Count"),
        }
    }
}

pub struct ResponsibilityCalculator {
    game: Game,
    weight_type: WeightType,
    thread_count: usize,
    step_size: usize,
    pub state_groups: StateGroups,
    silent: bool,
    pub sampled_count: usize, // If probabilistic sampling is used, this contains the number of samples after sampling has finished
    responsibility_version: ResponsibilityVersion,
}

pub struct ResponsibilityThreadState {
    results: Vec<ResponsibilityResult>,
    thread_index: usize,
}

impl ResponsibilityThreadState {
    pub fn new(state_groups: &StateGroups, thread_index: usize) -> Self {
        let mut results = Vec::with_capacity(state_groups.len());
        for i in 0..state_groups.len() {
            results.push(ResponsibilityResult::new(i, state_groups.len()));
        }
        Self {
            results,
            thread_index,
        }
    }
}

impl ResponsibilityCalculator {
    pub fn new(
        game: Game,
        thread_count: usize,
        weight_type: WeightType,
        state_grouping: StateGroups,
        responsibility_version: ResponsibilityVersion,
    ) -> Self {
        Self {
            game,
            weight_type,
            thread_count,
            step_size: 1024,
            state_groups: state_grouping,
            silent: false,
            sampled_count: 0,
            responsibility_version,
        }
    }

    pub fn set_silent(&mut self, silent: bool) {
        self.silent = silent;
    }

    fn compute_optimistic_responsibility(&mut self) -> Vec<ResponsibilityResult> {
        for state in &mut self.game.states {
            if state.default_owner != Player::Path {
                state.default_owner = Player::Safe;
                state.owner = Player::Safe;
            }
        }

        let mut res = Vec::new();

        for group in 0..self.state_groups.len() {
            self.state_groups.add_to_coalition(&mut self.game, group);
            let winner = self.game.determine_winner();
            if winner == Player::Safe {
                res.push(ResponsibilityResult::new(group, self.state_groups.len()));
            }
            self.state_groups
                .remove_from_coalition(&mut self.game, group);
        }

        let number_winning = res.len();
        let number_losing = self.state_groups.len() - number_winning;
        let losing_permutations = (1..=number_losing).fold(BigInt::one(), |l, r| l * r);
        for size in 1..=number_losing + 1 {
            let included_permutations = (1..=size - 1).fold(BigInt::one(), |l, r| l * r);
            let excluded_permutations =
                (1..=number_losing - (size - 1)).fold(BigInt::one(), |l, r| l * r);
            let total = &losing_permutations / (&included_permutations * &excluded_permutations);
            for resp in &mut res {
                resp.count_by_size[size] = BigRational::from_integer(total.clone());
            }
        }

        let weights =
            ResponsibilityResult::compute_weights(self.weight_type, self.state_groups.len());

        for resp in &mut res {
            resp.compute_values(&weights);
        }

        res
    }

    pub fn compute_individual_responsibility(&mut self) -> Vec<ResponsibilityResult> {
        if self.responsibility_version == ResponsibilityVersion::Optimistic {
            return self.compute_optimistic_responsibility();
        }
        let mut game_solver = game_solving::CachedGameSolver::new(
            self.game.clone(),
            self.thread_count,
            &self.state_groups,
        );
        game_solver.set_silent(self.silent);
        game_solver.prepare();

        let group_count = game_solver.state_groups.len();

        let start_responsibility_time = std::time::Instant::now();

        let mut thread_states = Vec::with_capacity(self.thread_count);
        for i in 0..self.thread_count {
            thread_states.push(ResponsibilityThreadState::new(&game_solver.state_groups, i));
        }

        let coalition_count = 1 << group_count;
        let coalitions = std::sync::Mutex::new((0..coalition_count).step_by(self.step_size));
        let progress_reporter =
            std::sync::Mutex::new(ProgressReporter::new(coalition_count, self.silent));

        thread_states.par_iter_mut().for_each(|thread_state| loop {
            let first_coalition = coalitions.lock().unwrap().next();
            if first_coalition.is_none() {
                break;
            }
            let first_coalition = first_coalition.unwrap();
            for base_coalition in
                first_coalition..(first_coalition + self.step_size as u64).min(coalition_count)
            {
                let size = base_coalition.count_ones() as usize;
                if !game_solver.is_game_winning(base_coalition) {
                    for added_state in 0..group_count {
                        let coalition = base_coalition | 1 << added_state;
                        if coalition != base_coalition && game_solver.is_game_winning(coalition) {
                            thread_state.results[added_state].count_by_size[size + 1] +=
                                BigRational::one();
                        }
                    }
                }
            }

            if thread_state.thread_index == 0 {
                progress_reporter
                    .lock()
                    .unwrap()
                    .set_current_coalition(first_coalition);
            }
        });
        progress_reporter.lock().unwrap().set_finished();

        let mut results = Vec::with_capacity(group_count);
        let weights = ResponsibilityResult::compute_weights(self.weight_type, group_count);

        for i in 0..game_solver.state_groups.len() {
            let mut result = ResponsibilityResult::new(i, group_count);
            for thread_state in &thread_states {
                result.add_counts(&thread_state.results[i]);
            }
            result.compute_values(&weights[..]);
            results.push(result);
        }

        if !self.silent {
            println!(
                "Computed responsibility in {:.2?}.",
                start_responsibility_time.elapsed()
            );
            println!();
        }
        results
    }

    pub fn sample_individual_responsibilities(
        &mut self,
        sample_target: SampleTarget,
    ) -> Vec<ResponsibilityResult> {
        let start_time = std::time::Instant::now();

        if self.responsibility_version != ResponsibilityVersion::Pessimistic {
            panic!("The randomized algorithm only supports pessimistic responsibility. For optimistic responsibility, it provides no benefit over the exact algorithm.")
        }

        if !self.silent {
            println!(
                "Sampling significant coalitions (there are {} state (groups))",
                self.state_groups.len()
            );
        }

        let mut thread_states = Vec::with_capacity(self.thread_count);
        for i in 0..self.thread_count {
            thread_states.push(SamplerState::new(
                self.game.clone(),
                &self.state_groups,
                sample_target.split_across_threads(self.thread_count, i),
                self.thread_count,
            ));
        }
        if !self.silent {
            thread_states[0].add_progress_bar(sample_target);
        }

        let samples_per_winning = 50;
        thread_states
            .par_iter_mut()
            .for_each(|s| s.run(samples_per_winning));

        if !self.silent {
            println!("Collecting thread results.");
        }

        let result = thread_states
            .into_par_iter()
            .reduce_with(|mut x, y| {
                x.add_values(&y);
                x
            })
            .unwrap();
        if !self.silent {
            println!(
                "Sampled {} coalitions in {:.2?}.",
                result.total_samples,
                start_time.elapsed()
            );
        }
        self.sampled_count = result.total_samples;

        result.to_responsibility_results(self.weight_type, samples_per_winning)
    }
}

pub struct TrimmedResponsibilityResult {
    pub group_index: usize,
    _n: usize,
    pub total_value: BigRational,
}

impl TrimmedResponsibilityResult {
    pub fn from_responsibility_result(responsibility_result: ResponsibilityResult) -> Self {
        Self {
            group_index: responsibility_result.group_index,
            _n: responsibility_result.n,
            total_value: responsibility_result.total_value,
        }
    }
}

pub struct ResponsibilityResult {
    pub group_index: usize,
    n: usize,
    count_by_size: Vec<BigRational>,
    value_by_size: Vec<BigRational>,
    pub total_value: BigRational,
}

impl ResponsibilityResult {
    pub fn new(group_index: usize, size: usize) -> Self {
        Self {
            group_index,
            n: size,
            count_by_size: vec![BigRational::zero(); size + 1],
            value_by_size: vec![BigRational::zero(); size + 1],
            total_value: BigRational::zero(),
        }
    }

    pub fn add_counts(&mut self, other: &ResponsibilityResult) {
        if self.n != other.n {
            panic!("Can only add counts to responsibility set if both sets have the same size");
        }
        for (self_count, other_count) in self
            .count_by_size
            .iter_mut()
            .zip(other.count_by_size.iter())
        {
            *self_count += other_count;
        }
    }

    pub fn compute_weights(weight_type: WeightType, n: usize) -> Vec<BigRational> {
        let mut factorials = Vec::with_capacity(n + 1);
        let mut current_value = BigInt::one();
        factorials.push(BigInt::one());
        for i in 1..=n {
            current_value *= i;
            factorials.push(current_value.clone());
        }

        let mut weights = Vec::with_capacity(n + 1);
        weights.push(BigRational::zero());
        for i in 1..=n {
            weights.push(match weight_type {
                WeightType::Shapley => BigRational::new(
                    factorials[n - i].clone() * factorials[i - 1].clone(),
                    factorials[n].clone(),
                ),
                WeightType::Banzhaf => {
                    BigRational::new(1.into(), BigInt::from(2).pow(n as u32 - 1))
                }
                WeightType::Count => BigRational::one(),
            })
        }
        weights
    }

    pub fn compute_values(&mut self, weights: &[BigRational]) {
        for i in 1..=self.n {
            self.value_by_size[i] = self.count_by_size[i].clone() * weights[i].clone();
            self.total_value += self.value_by_size[i].clone();
        }
    }

    pub fn to_string(&self, ts: &TransitionSystem, state_groups: &StateGroups) -> String {
        let name = state_groups.get_name(self.group_index, ts);
        let value_f64 = self.total_value.to_f64();
        match value_f64 {
            Some(value) => format!("({}): {:.8}", name, value),
            None => format!("({}): {}", name, self.total_value),
        }
    }
}

#[derive(Copy, Clone)]
pub enum SampleTarget {
    Samples(usize),
    ElapsedTime(Duration),
}

impl SampleTarget {
    pub fn split_across_threads(&self, thread_count: usize, thread_index: usize) -> Self {
        match self {
            Self::Samples(sample_count) => {
                if thread_index < sample_count % thread_count {
                    Self::Samples(sample_count / thread_count + 1)
                } else {
                    Self::Samples(sample_count / thread_count)
                }
            }
            Self::ElapsedTime(duration) => Self::ElapsedTime(*duration),
        }
    }
}

pub struct SamplerState<'a> {
    game: Game,
    state_groups: &'a StateGroups,
    total_samples: usize,
    samples_per_weight_global: Vec<usize>, // Samples that count for all states (i.e. those where the winning coalition was insignificant)
    samples_per_weight_local: Vec<Vec<BigRational>>, // Samples that only count for individual states. Can be negative if a sample counts for all except this state
    significant_per_weight: Vec<Vec<usize>>,
    target: SampleTarget,
    progress_bar: Option<SamplesProgressReporter>,
    thread_count: usize,
    seed: u64,
}

impl<'a> SamplerState<'a> {
    pub fn new(
        game: Game,
        state_groups: &'a StateGroups,
        target: SampleTarget,
        thread_count: usize,
    ) -> SamplerState<'a> {
        let size = state_groups.len();
        let mut samples_per_weight_global = Vec::with_capacity(size + 1);
        let mut samples_per_weight_local = Vec::with_capacity(size + 1);
        let mut significant_per_weight = Vec::with_capacity(size + 1);
        for _ in 0..=size {
            samples_per_weight_global.push(0);
            samples_per_weight_local.push(vec![BigRational::zero(); size]);
            significant_per_weight.push(vec![0; size]);
        }

        Self {
            game,
            state_groups,
            total_samples: 0,
            samples_per_weight_global,
            samples_per_weight_local,
            significant_per_weight,
            target,
            progress_bar: None,
            thread_count,
            seed: fastrand::u64(..),
        }
    }

    pub fn add_progress_bar(&mut self, target: SampleTarget) {
        self.progress_bar = Some(match target {
            SampleTarget::Samples(samples) => {
                SamplesProgressReporter::new_with_count(samples as u64)
            }
            SampleTarget::ElapsedTime(duration) => {
                SamplesProgressReporter::new_with_duration((duration.as_millis() / 100) as u64)
            }
        });
    }

    pub fn get_samples(&self, size: usize, state: usize) -> BigRational {
        BigRational::from_integer(self.samples_per_weight_global[size].into())
            + &self.samples_per_weight_local[size][state]
    }

    pub fn run(&mut self, samples_per_winning: usize) {
        let start_time = std::time::Instant::now();
        fastrand::seed(self.seed);

        loop {
            let new_progress_value = match self.target {
                SampleTarget::Samples(sample_target) => {
                    if self.total_samples >= sample_target {
                        break;
                    }
                    (self.total_samples * self.thread_count) as u64
                }
                SampleTarget::ElapsedTime(target_duration) => {
                    if start_time.elapsed() >= target_duration {
                        break;
                    }
                    (start_time.elapsed().as_millis() / 100) as u64
                }
            };
            if let Some(progress_bar) = &mut self.progress_bar {
                progress_bar.set_current_size(new_progress_value);
            }
            self.sample(self.state_groups.len().min(samples_per_winning));
        }
        if let Some(progress_bar) = &mut self.progress_bar {
            progress_bar.finish();
            println!("Waiting for other processes to finish");
        }
    }

    pub fn sample(&mut self, samples_if_winning: usize) {
        let size = fastrand::usize(1..=self.state_groups.len());
        let samples_if_winning = samples_if_winning.min(size);

        let members = self.sample_coalition(size);

        for &member in &members {
            self.state_groups.add_to_coalition(&mut self.game, member);
        }

        let winner = self.game.determine_winner();

        if winner == Player::Safe {
            let mut remaining_samples = samples_if_winning;
            // The share of members sampled is samples/size, so each sample counts size/sample.
            // Because we add one global sample (see below), we need to add size/sample - 1 local
            // counts, which is equal to (size-sample)/sample
            let sample_factor =
                BigRational::new((size - remaining_samples).into(), remaining_samples.into());
            for (i, &member) in members.iter().enumerate() {
                if fastrand::f64() % 1.0 < remaining_samples as f64 / (size - i) as f64 {
                    remaining_samples -= 1;
                    self.state_groups
                        .remove_from_coalition(&mut self.game, member);

                    if self.game.determine_winner() != winner {
                        self.significant_per_weight[size][member] += 1;
                    }
                    self.samples_per_weight_local[size][member] += &sample_factor;
                    self.state_groups.add_to_coalition(&mut self.game, member);
                } else {
                    self.samples_per_weight_local[size][member] -= BigRational::one();
                }
            }
        }
        self.samples_per_weight_global[size] += 1;

        for &member in &members {
            self.state_groups
                .remove_from_coalition(&mut self.game, member);
        }

        self.total_samples += 1;
    }

    fn sample_coalition(&self, size: usize) -> Vec<usize> {
        let mut members = Vec::new();
        let mut remaining_members = size;
        for i in 0..self.state_groups.len() {
            if fastrand::f64() % 1.0
                <= remaining_members as f64 / (self.state_groups.len() - i) as f64
            {
                members.push(i);
                remaining_members -= 1;
            }
        }
        members
    }

    pub fn add_values(&mut self, other: &SamplerState) {
        self.total_samples += other.total_samples;
        for i in 0..self.samples_per_weight_global.len() {
            self.samples_per_weight_global[i] += other.samples_per_weight_global[i];
            for j in 0..self.significant_per_weight[i].len() {
                self.samples_per_weight_local[i][j] += &other.samples_per_weight_local[i][j];
                self.significant_per_weight[i][j] += other.significant_per_weight[i][j];
            }
        }
    }

    pub fn to_responsibility_results(
        &self,
        weight_type: WeightType,
        samples_per_winning: usize,
    ) -> Vec<ResponsibilityResult> {
        let mut results = Vec::with_capacity(self.state_groups.len());

        // The count of the total samples is scaled because we only sample a proportion of the
        // members of each coalition. The number of significant samples is not scaled (to reduce
        // the amount of BigRational math we need to do). Therefore, we still need to scale it using
        // the factors we compute in the following:
        let mut significant_factor = Vec::with_capacity(self.state_groups.len() + 1);
        significant_factor.push(BigRational::one());
        for size in 1..=self.state_groups.len() {
            let samples = size.min(samples_per_winning);
            significant_factor.push(BigRational::new(size.into(), samples.into()));
        }

        if weight_type == WeightType::Shapley {
            // In the shapley case, we use the following optimisation:
            // Instead of first estimating the count per size and weights per size and using this to
            // calculate the value per size, we directly estimate the weight per size. This works
            // because the weight per size and count per size almost perfectly cancel out.
            // For each size, we have
            //     significant_samples/total_samples * coalitions_of_size * shapley_weight_per_item
            //   = significant_samples/total_samples / size

            for index in 0..self.state_groups.len() {
                let mut result = ResponsibilityResult::new(index, self.state_groups.len());
                for size in 1..=self.state_groups.len() {
                    let samples = self.get_samples(size, index);
                    if samples.is_positive() {
                        result.value_by_size[size] = BigRational::new(
                            self.significant_per_weight[size][index].into(),
                            size.into(),
                        ) * &significant_factor[size]
                            / &samples;

                        result.total_value += &result.value_by_size[size];
                        // println!(
                        //     "Group {}, size {}: {} * {} significant out of {}, value: {}",
                        //     index,
                        //     size,
                        //     self.significant_per_weight[size][index],
                        //     significant_factor[size],
                        //     samples,
                        //     result.value_by_size[size]
                        // );
                    }
                }

                results.push(result);
            }
        } else {
            let mut factorials = Vec::with_capacity(self.state_groups.len() + 1);
            let mut current_value = BigInt::from(1);
            factorials.push(current_value.clone()); // For n=0
            factorials.push(current_value.clone()); // For n=1
            for i in 2..=self.state_groups.len() {
                current_value *= i;
                factorials.push(current_value.clone());
            }

            let mut factors = Vec::with_capacity(self.state_groups.len() + 1);
            for size in 0..=self.state_groups.len() {
                let mut factor_entry = Vec::with_capacity(self.state_groups.len());

                let coalitions_of_size = factorials[self.state_groups.len()].clone()
                    / (factorials[size].clone()
                        * factorials[self.state_groups.len() - size].clone());
                for state in 0..self.state_groups.len() {
                    let samples = self.get_samples(size, state);
                    let factor = if samples.is_positive() {
                        samples.recip() * &coalitions_of_size * &significant_factor[size]
                    } else {
                        BigRational::one() // Value doesn't matter, as it will always be multiplied by 0. This check just exists to ensure it is not NaN.
                    };
                    factor_entry.push(factor);
                }
                factors.push(factor_entry);
            }

            let weights =
                ResponsibilityResult::compute_weights(weight_type, self.state_groups.len());

            for i in 0..self.state_groups.len() {
                let mut result = ResponsibilityResult::new(i, self.state_groups.len());
                for size in 1..=self.state_groups.len() {
                    result.count_by_size[size] = factors[size][i].clone()
                        * BigInt::from(self.significant_per_weight[size][i]);
                }

                result.compute_values(&weights[..]);
                results.push(result);
            }
        }

        results
    }
}

fn is_subset_of<T: std::ops::BitOr + Copy + From<u8>>(potential_subset: T, set: T) -> bool
where
    <T as std::ops::BitOr>::Output: std::ops::BitXor<T>,
    <<T as std::ops::BitOr>::Output as std::ops::BitXor<T>>::Output: PartialEq<T>,
{
    ((potential_subset | set) ^ set) == T::from(0)
}

struct SamplesProgressReporter {
    size_bar: indicatif::ProgressBar,
}

impl SamplesProgressReporter {
    pub fn new_with_count(max_size: u64) -> Self {
        let size_bar = indicatif::ProgressBar::new(max_size);
        size_bar.set_style(
            indicatif::ProgressStyle::with_template("Coalitions: {pos}/{len} [{wide_bar}]")
                .unwrap()
                .progress_chars("=> "),
        );
        Self { size_bar }
    }
    pub fn new_with_duration(duration_in_deciseconds: u64) -> Self {
        let size_bar = indicatif::ProgressBar::new(duration_in_deciseconds);
        let duration = duration_in_deciseconds as f64 * 0.1;
        let style_string = format!("Elapsed: {{elapsed}}/{}s [{{wide_bar}}]", duration);
        size_bar.set_style(
            indicatif::ProgressStyle::with_template(style_string.as_str())
                .unwrap()
                .progress_chars("=> "),
        );
        Self { size_bar }
    }

    pub fn set_current_size(&mut self, size: u64) {
        if size != self.size_bar.position() {
            self.size_bar.set_position(size);
        }
    }

    pub fn finish(&mut self) {
        self.size_bar.finish();
    }
}

struct ProgressReporter {
    coalition_bar: Option<indicatif::ProgressBar>,
    last_update: std::time::Instant,
}

impl ProgressReporter {
    pub fn new(coalition_count: u64, silent: bool) -> Self {
        if silent {
            Self {
                coalition_bar: None,
                last_update: std::time::Instant::now(),
            }
        } else {
            let coalition_bar = indicatif::ProgressBar::new(coalition_count);
            coalition_bar.set_style(
                indicatif::ProgressStyle::with_template("Coalition: {pos}/{len} [{wide_bar}]")
                    .unwrap()
                    .progress_chars("=> "),
            );
            Self {
                coalition_bar: Some(coalition_bar),
                last_update: std::time::Instant::now(),
            }
        }
    }

    pub fn set_current_coalition(&mut self, coalition: u64) {
        if let Some(coalition_bar) = &self.coalition_bar {
            if self.last_update.elapsed().as_millis() > 100 {
                coalition_bar.set_position(coalition);
            }
        }
    }

    pub fn set_finished(&mut self) {
        if let Some(coalition_bar) = &self.coalition_bar {
            coalition_bar.finish();
        }
    }
}
