use pdf_presenter::{desktop, poc::Options};

fn main() {
    let options = match Options::parse(std::env::args().skip(1)) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    };
    if options.help {
        println!(
            "PDF Presenter {} - dual-screen proof of concept\n\
Usage: pdf-presenter [--presenter N] [--audience N] [--windowed]\n\
       pdf-presenter --list-monitors\n\
       pdf-presenter --smoke-test\n\
Default: both fullscreen; presenter on primary, mock output on secondary.\n\
One display: views overlap; use --windowed for rehearsal. Display numbers start at 1.\n\
Keys: arrows/space slides, B blank, S swap, P/A cycle displays,\n\
F audience fullscreen, V presenter fullscreen, Esc safe windowed mode,\n\
R refresh displays, Q quit. No PDF loading or sleep inhibition yet.",
            env!("CARGO_PKG_VERSION")
        );
        return;
    }
    if let Err(error) = desktop::run(options) {
        eprintln!("PDF Presenter could not start/continue: {error}");
        std::process::exit(1);
    }
}
