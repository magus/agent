mod app;
mod devices;
mod platform;
mod protocol;
mod run;
mod service;

fn main() {
    if let Err(error) = run::run() {
        eprintln!("Agent: {error}");
        std::process::exit(1);
    }
}
