fn main() {
    if let Err(error) = agent_client::run() {
        eprintln!("Agent: {error}");
        std::process::exit(1);
    }
}
