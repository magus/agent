use crate::client::Args;
use std::io;

pub fn run() -> io::Result<()> {
    let argument_list: Vec<String> = std::env::args().skip(1).collect();
    if let [command, path] = argument_list.as_slice()
        && command == "--device"
    {
        return crate::devices::run_client(path);
    }
    if argument_list == ["--service-install"] {
        return crate::service::install();
    }
    if argument_list == ["--service"] {
        return crate::service::run();
    }
    if let [command, path] = argument_list.as_slice()
        && command == "--service-device"
    {
        let directory = crate::platform::runtime_dir()?.join("service");
        return crate::devices::run_client_at(path, &directory);
    }
    let args = Args::parse(&argument_list)?;

    if args == Args::Help {
        println!(
            "Usage: Agent [--setup | --chatgpt | --devices | --device REQUEST.json | --service | --service-install | --service-device REQUEST.json]"
        );
        return Ok(());
    }

    Err(io::Error::other(
        "Open Agent.app to use the native interface.",
    ))
}
