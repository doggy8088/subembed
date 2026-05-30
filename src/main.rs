use std::process::ExitCode;

fn main() -> ExitCode {
    match subembed::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}
