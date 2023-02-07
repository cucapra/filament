# Filament 2.0

## New Syntax from Paper

- `component` -> `comp`
- Delay as a concrete number on events: `G: 3`
- Interface ports only mention the event and not the delay: `@interface[G]`

## Polymorphism

Allow the availability intervals to be affected by parameters of the module:
```
comp Shift[N]<G: 1>(@[G, G+1] in: 32) -> (@[G+N, G+N+1] out: 32);
```

As well as the delay of events:
```
comp Div[W]<G: W>(@[G, G+1] left: W, @[G, G+1] right: W) -> (@[G+W, G+W+1] out: 32);
```

> Note: This reflects the constraints of an iterative divider which needs to iterate based on the width of the input.


## Meta Programming Constructs

Components can use meta-level `for` and `if` to generate code:
```
comp AddOrSub[A]<G: 1>(@[G, G+1] in: 32) -> (@[G, G+1] out: 32) {
    generate if A == 0 {
        A := new Add[32];
        a0 := A<G>(in, in);
        out = a0.out;
    } else {
        S := new Sub[32];
        s0 := S<G>(in, in);
        out = s0.out;
    }
}
```

## Component Parameterization

Component can be parameterized but there is no way to use the parameterization.

```
comp Div[W](@[G, G+1] in: 32) -> (@[G, G+1] out: 32) {
    // Wires are arrays that can use special index types that specify their
    // values based on the index in the array.
    bundle f[10]: forall i. @[G+i, G+i+1] 32;
    for (i upto W) {
        if (i == 0) {
            i := Init<G>(in);
            f{0} = i.out; // [G, G+1]
        } else {
            n := new Next<G+i>(f{i-1});
            f{i} = n.out; // [G+i, G+i+1]
        }
    }
}
```

### `bundle`

Wires are useful for connecting instances generated in a loop. For example, a chain of components can use the following type:
```
wires f[10]: for i. @[G+i, G+i+1] 32;
```

This describes a bundle of wires where the value at index `i` is available during the cycles `[G+i, G+i+1]` and has a width of 32 bits.

> **Note**: It should be possible to infer the type of a wire bundle from the connections made to it. It seems slightly complicated because we'd need to generalize types like `i == 0. [G, G+1]` and `i==1 [G+1, G+2]` into `forall i. @[G+i, G+i+1]`.


## Schedule Inference

Often times, the a Filament program is sufficiently constrainted that the schedule can be inferred. For example, the following program:
```
comp MultAdd<G: 1>(
    @[G, G+1] a: 32,
    @[G, G+1] b: 32,
    @[G+1, G+2] c: 32
) -> (@[G+1, G+2] out: 32) {
    m := new Mult<G+?>(a, b); // Assume this takes one cycle
    a := new Add<G+?>(m.out, c);
    out = a.out;
}
```

Instead of scheduling the instances for `Mult` and `Add` at specific times, we can use the `?` operator to indicate that the schedule should be inferred.
The schedule inference should (hopefully) infer that the `Mult` is scheduled at `G` and the `Add` is scheduled at `G+1`.

### Mixing with Polymorphism

What if we want to keep the latency of the `Mult` operation *abstract*; that is, the compiler gets to decide what exact latency should be used for the `Mult` operation later on.
Equivalently, maybe our language supports higher-order components and we want to pass in a `Mult` component that we don't know the latency of.

Keeping the `Mult` latency abstract would require that we also parameterize the interval for `c` and the `out`.

```
comp MultAdd[ML]<G: 1>( // ML is the latency of the mult
    @[G, G+1] a: 32,
    @[G, G+1] b: 32,
    @[G+ML, G+ML+1] c: 32
) -> (@[G+ML, G+ML+1] out: 32) {
    m := new Mult<G+?>(a, b); // Assume this takes one cycle
    a := new Add<G+?>(m.out, c);
    out = a.out;
}
```

The schedule inference problem for this program is more challenging: we need to infer that the `Add` is scheduled at `G+ML`.
Of course, `Add`'s module signature could probably help with this:
```
comp Add<G: 1>(@[G, G+1] a: 32, @[G, G+1] b: 32) -> (@[G, G] out: 32);
```

In this case, `Add`'s signature is straightforward but in general, it could itself contain other parameters that need to be inferred.

> Note: This program has an undocumented relationship between `ML` and the latency of the `Mult`. In a higer-order Filament, this program could probably be written as:
> ```
> comp MultAdd[ML]<G: 1>( // ML is the latency of the mult
>     @[G, G+1] a: 32,
>     @[G, G+1] b: 32,
>     @[G+ML, G+ML+1] c: 32
> ) -> (@[G+ML, G+ML+1] out: 32) {
>     ref comp Mult[ML]<G: 1>(@[G, G+1] a: 32, @[G, G+1] b: 32) -> (@[G, G+ML] out: 32);
>     m := new Mult<G+?>(a, b); // Assume this takes one cycle
>     a := new Add<G+?>(m.out, c);
>     out = a.out;
> }
> ```
>
> The `ref` keyword indicates that the component is a reference to a component that is defined elsewhere and needs to be provided to `MultAdd` when it is instantiated.


## Exact Pipelines

Filament's definition of pipelining requires that a component be schedulable *any* time after the initiation interval passes.
A different definition of pipelining would allow the component to be scheduled *exactly* at the initiation interval.
This can be useful since it allows the component to have a lower initiation interval than the current definition allows in cases of resource sharing.
The trade-off is that such pipelines are strictly less composable since they must be schedule at an integer multiple of the initiation interval.

We can allow Filament to specify that certain pipelines are *exact*, i.e., they use a different definition of pipelining.
All traditional pipelines (which adhere to the existing definition of pipelining in Filament) are exact but the vice-versa is not true.
This means that traditional pipelines can be used within exact pipelines but not the other way around.

Other constraints:
- The initiation interval of the exact pipeline must be an exact factor of the initiation intervals of all sub-pipelines (safe pipelining).