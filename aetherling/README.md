## Aetherling Evaluation

Evaluates Filament against Aetherling designs. The benchmarks are:
1. `sharpen`: Implementation of the unsharp filter from the Aetherling paper.
2. `conv2d`: A 2D convolutional filter.

Each benchmark has 7 different design points with varying throughput and latencies.

### Sharpen

The Aetherling harness checks that the output looks like:

```
X  X  X  X
X  X  X  X
X  X  65 74
X  X  93 98
```

This can be validated by running the following command:

```
stack ghci --test
> sharpen_output
```

Aetherling's harness treats the value `253` specially and ignores output values when the golden output uses `253` in a particular position.

### Conv2d

The Aetherling harness checks that the output looks like:

```
X  X  X  X
X  X  X  X
X  X  6  7
X  X  10 11
```

This can be validated by running:

```
stack ghci --test
> conv_2d_output
```