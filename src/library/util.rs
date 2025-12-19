use chrono::Utc;

pub fn log_time(msg: &str, verbose: bool) {
    if verbose {
        eprintln!("{}: {}", Utc::now(), msg);
    }
}
