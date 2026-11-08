use std::io;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Args {
    Setup,
    ChatGpt,
    Devices,
    Permissions,
    Help,
}

impl Args {
    pub(crate) fn parse(argument_list: &[String]) -> io::Result<Self> {
        match argument_list {
            [] => Ok(Self::Setup),
            [argument] => match argument.as_str() {
                "--setup" => Ok(Self::Setup),
                "--chatgpt" => Ok(Self::ChatGpt),
                "--devices" => Ok(Self::Devices),
                "--permissions" => Ok(Self::Permissions),
                "--help" | "-h" => Ok(Self::Help),
                _ => Err(io::Error::other("Unsupported arguments; use --help")),
            },
            _ => Err(io::Error::other("Unsupported arguments; use --help")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_existing_commands_and_rejects_conflicting_arguments() {
        assert_eq!(Args::parse(&[]).unwrap(), Args::Setup);

        for (argument, expected) in [
            ("--setup", Args::Setup),
            ("--chatgpt", Args::ChatGpt),
            ("--permissions", Args::Permissions),
            ("--help", Args::Help),
            ("-h", Args::Help),
        ] {
            assert_eq!(Args::parse(&[argument.into()]).unwrap(), expected);
        }

        assert!(Args::parse(&["--unknown".into()]).is_err());
        assert!(Args::parse(&["--setup".into(), "--chatgpt".into()]).is_err());
    }
}
