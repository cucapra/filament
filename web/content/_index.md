+++
+++
Filament is a hardware description language (HDL) that uses types to ensure that your hardware pipelines are composed correctly.
Filament's type system is very different from [every][bluespec] [other][chisel] [typed HDL][clash]: it uses a [Rust]-inspired type system to reason about structural hazards in your designs and eliminates them *at compile-time*.
For examples of the kinds of problems Filament can solve, take a look at our [tutorial][].

[bluespec]: https://bluespec.com/
[chisel]: https://www.chisel-lang.org/
[clash]: https://clash-lang.org/
[rust]: https://rust-lang.org
[tutorial]: /docs