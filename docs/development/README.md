# Development

This is a standalone, single-member Rust workspace. Install Rust with rustfmt,
Clippy and llvm-tools-preview, plus Python 3.11+ for CI helpers. The mock desktop uses
winit, softbuffer and an embedded bitmap font. PDF backend
and production presenter dependencies remain to be implemented against the hard
dual-screen, startup and responsiveness requirements.

- [Implementation and acceptance plan](plan.md)
- [Local checks and Jenkins](ci.md)
- [Persistent interfaces](../interfaces/README.md)
- [Changelog](../changelog.md)

Run `cargo run --locked -p pdf-presenter` for the native mock-window test.
See the [dual-screen proof guide](dual-screen-poc.md) for controls, limits and the
physical acceptance still required. PDF loading and release bundles remain pending.

- [Ordered implementation tasks](tasks.md)
- [Implementation choices and performance budgets](implementation.md)

Release target: a portable executable for each supported OS/architecture, bundling
all non-system dependencies. Runtime installation/downloads are not acceptable;
see implementation choices and task T25 for packaging and clean-machine tests.
