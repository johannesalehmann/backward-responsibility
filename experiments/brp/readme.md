# BRP -- grouped and individual

You can reproduce the results from the "Example: BRP" in the "State grouping" subsection of the implementation as follows.

To check that the stochastic algorithm does not produce significant results even after running for 60s, run

    ./target/release/bw-responsibility --benchmark experiments/brp/benchmark_individual_reproducible -j8

This will take tens of minutes, depending on your machine. After that, you should be able to see that the standard deviation for 60s is in parentheses, indicating a non-significant result.

To compare this with the grouped case, run

    ./target/release/bw-responsibility --benchmark experiments/brp/benchmark_grouped_reproducible -j8

This should not take as long. You should see a standard deviation of 0.0006 for t=1s.