//! Fixed, redaction-safe operator rendering for the resident authority.
//!
//! The monitor is intentionally independent of ambient environment filters:
//! a Root Authority always receives `INFO`, `WARN`, and `ERROR` on stderr.
//! It is only a live rendering. It does not persist trace lines, interpret
//! protocol bodies, or receive credentials, raw frames, or actor context.

use thiserror::Error;
use tracing::Level;

#[derive(Debug, Error)]
pub enum MonitorInstallError {
    #[error("a process-wide tracing subscriber is already installed")]
    AlreadyInstalled,
}

/// Installs the non-optional stderr monitor before daemon startup.
///
/// The filter is compiled into trusted physics rather than read from
/// `RUST_LOG`: diagnostics may never suppress this operational surface.
pub fn install_mandatory_monitor() -> Result<(), MonitorInstallError> {
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_ansi(false)
        .without_time()
        .compact()
        .with_writer(std::io::stderr)
        .try_init()
        .map_err(|_| MonitorInstallError::AlreadyInstalled)
}

#[cfg(test)]
mod tests {
    use std::{
        io::{self, Write},
        sync::{Arc, Mutex},
    };

    use tracing::Level;
    use tracing_subscriber::fmt::MakeWriter;

    #[derive(Clone, Default)]
    struct CapturedMonitor(Arc<Mutex<Vec<u8>>>);

    struct CapturedWriter(Arc<Mutex<Vec<u8>>>);

    impl Write for CapturedWriter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0
                .lock()
                .expect("test monitor lock")
                .extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl<'writer> MakeWriter<'writer> for CapturedMonitor {
        type Writer = CapturedWriter;

        fn make_writer(&'writer self) -> Self::Writer {
            CapturedWriter(Arc::clone(&self.0))
        }
    }

    #[test]
    fn mandatory_monitor_renders_info_and_higher_but_not_diagnostics() {
        let captured = CapturedMonitor::default();
        let subscriber = tracing_subscriber::fmt()
            .with_max_level(Level::INFO)
            .with_ansi(false)
            .without_time()
            .compact()
            .with_writer(captured.clone())
            .finish();

        tracing::subscriber::with_default(subscriber, || {
            tracing::trace!(target: "society.ledger", "trace-marker");
            tracing::debug!(target: "society.ledger", "debug-marker");
            tracing::info!(target: "society.ledger", "info-marker");
            tracing::warn!(target: "society.ledger", "warn-marker");
            tracing::error!(target: "society.ledger", "error-marker");
        });

        let rendered = String::from_utf8(captured.0.lock().expect("test monitor lock").clone())
            .expect("monitor emits UTF-8");
        assert!(rendered.contains("info-marker"));
        assert!(rendered.contains("warn-marker"));
        assert!(rendered.contains("error-marker"));
        assert!(!rendered.contains("debug-marker"));
        assert!(!rendered.contains("trace-marker"));
    }
}
