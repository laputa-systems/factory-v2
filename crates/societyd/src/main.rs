use std::{
    env,
    os::{fd::FromRawFd, unix::net::UnixStream},
    process::ExitCode,
};

use societyd::{Daemon, DaemonConfig, install_mandatory_monitor};

fn main() -> ExitCode {
    if let Err(error) = install_mandatory_monitor() {
        eprintln!("societyd failed to install mandatory monitor: {error}");
        return ExitCode::from(1);
    }
    let mut arguments = env::args_os();
    let _program = arguments.next();
    let (Some(flag), Some(runtime_root)) = (arguments.next(), arguments.next()) else {
        eprintln!(
            "usage: societyd --runtime-root <path> [--supervisor-fd <inherited-unix-stream-fd>]\n\
             requires SOCIETY_DATABASE_URL; optionally set SOCIETY_DATABASE_MIGRATION_URL and SOCIETY_DATABASE_SCHEMA"
        );
        return ExitCode::from(2);
    };
    if flag != "--runtime-root" {
        eprintln!(
            "usage: societyd --runtime-root <path> [--supervisor-fd <inherited-unix-stream-fd>]\n\
             requires SOCIETY_DATABASE_URL; optionally set SOCIETY_DATABASE_MIGRATION_URL and SOCIETY_DATABASE_SCHEMA"
        );
        return ExitCode::from(2);
    }
    let config = match (arguments.next(), arguments.next(), arguments.next()) {
        (None, None, None) => DaemonConfig::new(runtime_root),
        (Some(supervisor_flag), Some(descriptor), None) if supervisor_flag == "--supervisor-fd" => {
            let Some(descriptor) = descriptor
                .to_str()
                .and_then(|value| value.parse::<i32>().ok())
            else {
                eprintln!("societyd requires a numeric inherited supervisor fd");
                return ExitCode::from(2);
            };
            if descriptor < 3 {
                eprintln!("societyd refuses standard-stream descriptors as supervisor authority");
                return ExitCode::from(2);
            }
            // Check liveness before `from_raw_fd`: wrapping an invalid integer
            // would violate that constructor's ownership precondition. Socket
            // family, connected peer, same-user attribution, and CLOEXEC are
            // then checked by `Daemon::bind` before this authority can serve.
            // SAFETY: `F_GETFD` reads flags only; it does not transfer or close
            // the supplied numeric descriptor.
            if unsafe { libc::fcntl(descriptor, libc::F_GETFD) } < 0 {
                eprintln!("societyd received an invalid inherited supervisor fd");
                return ExitCode::from(2);
            }
            // SAFETY: the process supervisor explicitly inherited this owned
            // Unix-stream descriptor into `societyd`; ownership transfers to
            // `UnixStream`, which closes it when the daemon stops.
            let stream = unsafe { UnixStream::from_raw_fd(descriptor) };
            DaemonConfig::new(runtime_root).with_supervisor_stream(stream)
        }
        _ => {
            eprintln!(
                "usage: societyd --runtime-root <path> [--supervisor-fd <inherited-unix-stream-fd>]\n\
                 requires SOCIETY_DATABASE_URL; optionally set SOCIETY_DATABASE_MIGRATION_URL and SOCIETY_DATABASE_SCHEMA"
            );
            return ExitCode::from(2);
        }
    };
    let mut daemon = match Daemon::bind(config) {
        Ok(daemon) => daemon,
        Err(error) => {
            eprintln!("societyd failed to bind: {error}");
            return ExitCode::from(1);
        }
    };
    let shutdown = match daemon.shutdown_handle().with_process_signals() {
        Ok(shutdown) => shutdown,
        Err(error) => {
            eprintln!("societyd failed to install signal bridge: {error}");
            return ExitCode::from(1);
        }
    };
    match daemon.serve_until(&shutdown) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("societyd stopped with error: {error}");
            ExitCode::from(1)
        }
    }
}
