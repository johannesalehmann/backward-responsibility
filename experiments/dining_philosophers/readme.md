# Grouped Dining Philosophers

To compute the grouped responsibility, run: 

    ./target/release/bw-responsibility --prism-model experiments/dining_philosophers/dining_philosophers.prism -c experiments/dining_philosophers/dining_philosophers.ce -b "sbar" -g

You should get the following responsibilities.

| State | Responsibility |
|-------|----------------|
| p1    | 0.25           |
| p2    | 0.25           |
| p3    | 0.25           |
| p4    | 0.25           |

The runtime should be in the order of 10^-3 seconds.