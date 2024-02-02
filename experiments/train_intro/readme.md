# Grouped Train Switches

This experiment is mentioned in the intuitive explanation of _State Grouping_ in the implementation section.

To compute the individual responsibilities, run:

    ./target/release/bw-responsibility --prism-model experiments/train_intro/train_intro.prism -c "experiments/train_intro/train_intro.ce" -b "sbar"

You should get the following responsibilities:

| State | Responsibility |
|-------|----------------|
| (s=2) | 0.66666667     |
| (s=1) | 0.16666667     |
| (s=3) | 0.16666667     |