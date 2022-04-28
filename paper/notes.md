## Changes for the Paper

### High Level Notes

1. Pedro says: Really have to sell people on the connection to linear types and separation logic.
  - Linear logic talks about resources that cannot be used again
  - Separation logic talks about "recoverable resources"
  - The danger is that their semantics are very closely related because their product and arrow types are closely related

### Paper Changes
1. Changing the types to have non-linear operation graded with intervals
2. Distinction between "producers" (arrow types with non-linearity) and "resources" (linear base types with a start time and latency).
3. Type system for signal types
  - Copy from Rachit's PLDG presentation and see if that "just works"
  - The start time of signals is the corresponds to the start time "tag" for linear resources.
4. Abstract vs. Concrete Syntax
  - A.S should have explicit forall quantifiers everywhere needed while C.S should not.
  - C.S should distinguish between "invocation scheduling" and "signal connecting" (c.f. Rachit's talk at PLDG).
  - A.S should keep "split" operation to make type checking syntax directed
  - "wrap" probably does not work since it abstracts the concrete resource. Get rid of it.
  - Do we need the `time t = T` command? Can't we just inline it into the `when` statements that use it? If yes, maybe keep it in the concrete syntax as sugar.
  - Think about how to present the signal types together with the resource types. Is it a good idea to replace big, unwieldy signal types with names like "ADD". It's probably the case that `add` and `sub` have the same signal types. Pedro says yes regardless.
5. Think about contexts and the operations that they need. With the new separation logic stuff, we might need to reconsider how the contexts need to work.



### Related Works
- Bounded linear logic in a resource semi-ring
- Graded Modal Types

## Concrete Programs

```
component mac<t>(
  @[t, t+2] left: 32,
  @[t, t+2] right: 32,
  @[t+2, t+3] acc: 48
) -> (
  @[t+3, t+4] out: 48
) {
  m = mult[];
  a = add[];
  when t { m0 = m(left, right) }
  when t+2 { a0 = a(m0.out, acc) }
  when t+3 { out = a0.out }
}
```

Adrian recommends changing `when` blocks to:

```
when t+2 {
  // Scheduling invocation
  a0 = invoke a[];
  // Connecting signals
  a0.left = m0.out;
  a0.right = acc;
}
```
