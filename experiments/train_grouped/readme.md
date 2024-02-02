# Grouped Train Switches

This experiment is mentioned in the intuitive explanation of _State Grouping_ in the implementation section.

To compute the individual responsibilities, run:

    ./target/release/bw-responsibility --prism-model experiments/train_grouped/train_grouped.prism -c "experiments/train_grouped/train_grouped.ce" -b "sbar"

You should get the following responsibilities:

| State | Responsibility |
|-------|----------------|
| (s=2) |  0.52380952    |
| (s=1) |  0.35714286    |
| (s=5) |  0.02380952    |
| (s=6) |  0.02380952    |
| (s=7) |  0.02380952    |
| (s=8) |  0.02380952    |
| (s=9) |  0.02380952    |


To compute grouped responsibilites, run:

./target/release/bw-responsibility --prism-model experiments/train_grouped/train_grouped.prism -c "experiments/train_grouped/train_grouped.ce" -b "sbar" -g

You should get the following responsibilities:

| State | Responsibility |
|-------|----------------|
| (s2)  | 0.66666667     |
| (s1)  | 0.16666667     |
| (t)   | 0.16666667     |