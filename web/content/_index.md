+++
+++
Filament is a hardware description language (HDL) that uses types to ensure that your hardware pipelines are composed correctly.
Filament's type system is very different from [every][bluespec] [other][chisel] [typed HDL][clash]: it uses a substructural (like Rust) to reason about structural hazards in your designs and eliminates them *at compile-time*.

[bluespec]: https://bluespec.com/
[chisel]: https://www.chisel-lang.org/
[clash]: https://clash-lang.org/