# Reproducing the results from the paper

This list contains instructions to reproduce all experiments conduced in the paper. It assumes that you have installed the tool. Otherwise, see the instructions at [../readme.md](../readme.md).

## General notes

Navigate to the root directory of the tool so that the binary is located at ```target/release/bw-responsibility```.

You can perform all experiments either with [PRISM](https://www.prismmodelchecker.org/) or without it. PRISM is only used to preprocess the transition system and generate the counterexample. If you do not have PRISM installed, append ```--no-prism``` to every command you execute. The tool will then use pre-processed transition systems and manually specified counterexamples instead. The results will be the same in either case.

## Reproducibility and randomness

Several benchmarks use a stochastic algorithm. In these cases, the tool is usually run for t seconds and as many samples as possible are performed. You can run the same benchmarks, but due to different hardware and different random seeds, the results might be different.

Therefore, we also provide a reproducible version of each benchmark. This contains a fixed seed and the number of samples that our machine was able to perform in the given time. The result should thus also match our results exactly. However, the runtime may differ and not match the runtime given in the paper.

We have also only performed limited testing on other hardware. It is possible that the randomness is not reproduced exactly if other platforms provide different implementations of randomness.

## Reproducing the results

To reproduce the train example from Figure 1 and Table 1, see [train_intro/readme.md](train_intro/readme.md).

To reproduce the Peg Solitaire analysis, see [peg_solitaire/readme.md](peg_solitaire/readme.md).

To reproduce the analysis of the misrouted train in Dresden, see [dresden_misrouted_train/readme.md](dresden_misrouted_train/readme.md).

To reproduce Table 2, see [stochastic_benchmarking/readme.md](stochastic_benchmarking/readme.md). Note that reproducing the entire table can take several hours or up to a day, depending on your hardware.

To reproduce the introductory example for state grouping, see [train_grouped/readme.md](train_grouped/readme.md).

To reproduce the dining philosophers example for state grouping, see [dining_philosophers]

## Peg Solitaire

## Misrouted train in Dresden

## Stochastic benchmarks

## Dining philosophers

## BRP