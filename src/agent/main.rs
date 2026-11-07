mod app;

use app::App;

fn main() {
    if let Err(error) = App::run() {
        eprintln!("Agent: {error}");
        std::process::exit(1);
    }
}
