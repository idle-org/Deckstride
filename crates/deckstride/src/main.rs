#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

use deckstride::{native, poc::Options};

fn main() {
    if std::env::args().nth(1).as_deref() == Some("--licenses") {
        print!("{}", include_str!("../../../THIRD_PARTY_NOTICES.txt"));
        return;
    }
    if std::env::args().nth(1).as_deref() == Some("--worker") {
        if let Err(e) = deckstride::worker::run() {
            eprintln!("{e}");
            std::process::exit(1);
        }
        return;
    }
    if std::env::args().nth(1).as_deref() == Some("--presentation-host") {
        if let Err(e) = native::run_output() {
            eprintln!("{e}");
            std::process::exit(1);
        }
        return;
    }
    if std::env::args().nth(1).as_deref() == Some("--keep-awake") {
        let _ = deckstride::output::keep_awake();
        return;
    }
    let mut args = std::env::args().skip(1).collect::<Vec<_>>();
    let proof = args.iter().any(|s| s == "--display-proof");
    args.retain(|s| s != "--display-proof");
    let path = if let Some(index) = args.iter().position(|s| s == "--open") {
        if index + 1 >= args.len() {
            eprintln!("--open needs a PDF or folder path");
            std::process::exit(2);
        }
        let path = args.remove(index + 1);
        args.remove(index);
        Some(std::path::PathBuf::from(path))
    } else if args.first().is_some_and(|s| !s.starts_with('-')) {
        Some(std::path::PathBuf::from(args.remove(0)))
    } else {
        None
    };
    let options = match Options::parse(args) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    };
    if options.help {
        println!(
            "Deckstride {} — native PDF and picture-folder presenter\n\
Usage: deckstride [--open PDF_OR_FOLDER] [--windowed]\n\
       deckstride --list-monitors\n\
       deckstride --presenter N --audience N\n\
       deckstride --smoke-test\n\
Both roles start fullscreen. Select/mirror screens from Displays.\n\
Keys: arrows/space navigate, number + Enter jumps, B blanks.\n\
Escape/Ctrl+Q request quit confirmation; Escape first reverts a display trial.\n\
Hold S for one second to swap Presenter/Slides groups.\n\
Fullscreen uses explicit controls; right-click restores hidden header/footer.\n\
All slides opens a windowed browser; click only selects unless enabled.\n\
PDF/image decoding runs in supervised workers; no external PDF backend needed.",
            env!("CARGO_PKG_VERSION")
        );
        return;
    }
    let result = if proof {
        deckstride::desktop::run(options)
    } else {
        native::run(options, path)
    };
    if let Err(error) = result {
        eprintln!("Deckstride could not start/continue: {error}");
        std::process::exit(1);
    }
}
