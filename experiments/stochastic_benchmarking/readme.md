# Stochastic Benchmarks

To reproduce Table 2, you can run the following command:

    ./target/release/bw-responsibility --benchmark experiments/stochastic_benchmarking/benchmarks_reproducible -j8

*Note that this will take a long time* (about 30-60 minutes per row on my machine). This will perform the same number of samples and use the same seed as the benchmark in the paper, so the result should be a perfect reproduction of the table in the paper. We have however not checked whether this produces reproducible results on different hardware configurations, (e.g. a different number of cores or a different architecture/operating system).

If you instead want to see how many samples your machine is able to make in the same time, run the following

    ./target/release/bw-responsibility --benchmark experiments/stochastic_benchmarking/benchmarks

If you want to reproduce a single line from the benchmark, you can use one of the following commands:

*alternating_bit*:

    ./target/release/bw-responsibility --benchmark experiments/stochastic_benchmarking/alternating_bit/benchmark_reproducible -j8

*brp, N=4, MAX=2*:

    ./target/release/bw-responsibility --benchmark experiments/stochastic_benchmarking/brp_N_4_MAX_2/benchmark_reproducible -j8

*brp, N=16, MAX=3*:

    ./target/release/bw-responsibility --benchmark experiments/stochastic_benchmarking/brp_N_16_MAX_3/benchmark_reproducible -j8

*crowds, TR=3, CS=5*:

    ./target/release/bw-responsibility --benchmark experiments/stochastic_benchmarking/crowds_TR_3_CS_5/benchmark_reproducible -j8

*dining_philosophers, N=3*:

    ./target/release/bw-responsibility --benchmark experiments/stochastic_benchmarking/dining_philosophers_3/benchmark_reproducible -j8

*dining_philosophers, N=5*:

    ./target/release/bw-responsibility --benchmark experiments/stochastic_benchmarking/dining_philosophers_5/benchmark_reproducible -j8

*dresden_railways*:

    ./target/release/bw-responsibility --benchmark experiments/stochastic_benchmarking/dresden_railways/benchmark_reproducible -j8

*generals, N=3*:

    ./target/release/bw-responsibility --benchmark experiments/stochastic_benchmarking/generals_3/benchmark_reproducible -j8

*generals, N=5*:

    ./target/release/bw-responsibility --benchmark experiments/stochastic_benchmarking/generals_5/benchmark_reproducible -j8

*generals, N=8*:

    ./target/release/bw-responsibility --benchmark experiments/stochastic_benchmarking/generals_8/benchmark_reproducible -j8
