fn main() {
    use std::io::{self, Write};

    let outcome = stellar_forge::run_cli(std::env::args_os());
    match outcome.stream {
        stellar_forge::OutputStream::Stdout => {
            let mut stdout = io::stdout();
            stdout
                .write_all(outcome.rendered.as_bytes())
                .expect("stdout write should succeed");
            stdout.flush().expect("stdout flush should succeed");
        }
        stellar_forge::OutputStream::Stderr => {
            let mut stderr = io::stderr();
            stderr
                .write_all(outcome.rendered.as_bytes())
                .expect("stderr write should succeed");
            stderr.flush().expect("stderr flush should succeed");
        }
    }

    if outcome.exit_code != 0 {
        std::process::exit(outcome.exit_code);
    }
}
