# Dresden Central Station

To compute responsibilities for the train station, run:

    ./target/release/bw-responsibility --prism-model experiments/dresden_misrouted_train/dresden_railways.prism -c experiments/dresden_misrouted_train/dresden_railways.ce -b "sbar"

You should get the following responsibilities:

| State  | Responsibility |
|--------|----------------|
| (t=35) | 0.75000000     |
| (t=36) | 0.08333333     |
| (t=37) | 0.08333333     |
| (t=42) | 0.08333333     |
