use std::process::ExitCode;

fn main() -> ExitCode {
    match burn_in_zh_subtitles::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}
