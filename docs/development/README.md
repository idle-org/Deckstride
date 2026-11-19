# Development

This is a standalone, single-member Rust workspace. Install Rust with rustfmt,
Clippy and llvm-tools-preview, plus Python 3.11+ for CI helpers. The foundation has
no third-party Rust dependencies. The desktop and PDF renderer dependencies remain
to be selected against the hard dual-screen requirement on Windows, macOS and Linux
and measured presentation speed; sharing Canvas Pilot's UI stack is optional.

- [Implementation and acceptance plan](plan.md)
- [Local checks and Jenkins](ci.md)
- [Persistent interfaces](../interfaces/README.md)
- [Changelog](../changelog.md)

Run `cargo run --locked -p pdf-presenter` for the bootstrap status executable.
No PDF opening, GUI, installation bundle or presentation output exists yet.

- [Ordered implementation tasks](tasks.md)
- [Implementation choices and performance budgets](implementation.md)

Release target: a portable executable for each supported OS/architecture, bundling
all non-system dependencies. Runtime installation/downloads are not acceptable;
see implementation choices and task T25 for packaging and clean-machine tests.
