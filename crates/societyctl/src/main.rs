use std::{env, process::ExitCode};

use societyctl::SocietyctlClient;
use societyd::protocol::CorrelationId;

fn main() -> ExitCode {
    let mut arguments = env::args_os();
    let _program = arguments.next();
    let (Some(flag), Some(socket_path), Some(command), None) = (
        arguments.next(),
        arguments.next(),
        arguments.next(),
        arguments.next(),
    ) else {
        eprintln!("usage: societyctl --socket <path> status");
        return ExitCode::from(2);
    };
    if flag != "--socket" || command != "status" {
        eprintln!("usage: societyctl --socket <path> status");
        return ExitCode::from(2);
    }
    let client = SocietyctlClient::connect(socket_path);
    match client.status(CorrelationId::new(1).expect("literal nonzero correlation")) {
        Ok(status) => {
            println!("{status:?}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("societyctl failed: {error}");
            ExitCode::from(1)
        }
    }
}
