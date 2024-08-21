
fn main() {
    inner::main_inner();
}

#[cfg(not(feature = "build-binaries"))]
mod inner {
    pub(crate) fn main_inner() {
        panic!("Binaries are disabled");
    }
}

#[cfg(feature = "build-binaries")]
mod inner {
    #![allow(unused_imports)]

    use std::error;
    use std::path::PathBuf;

    use quic_speed::deps::*;
    use quic_speed::bin_deps::*;
    use quic_speed::config::*;

    use quic_speed::server;

    use signal_hook::consts::signal::*;
    use signal_hook::iterator::Signals;

    use clap::Parser;
    use syslog::{Facility, Formatter3164, BasicLogger};
    use log::{SetLoggerError, LevelFilter, info, error, warn};

    #[derive(Parser, Debug, Clone)]
    #[command(version, about, long_about = None)]  
    struct Args {
        /// Path to the config file
        #[arg(short = 'c', long)]
        config: PathBuf,

        /// Enable verbose logging
        #[arg(short = 'v', long)]
        verbose: bool,

        /// Bind to a specific device
        #[arg(long)]
        bind_device: Option<String>,
    }

    pub(crate) fn main_inner() {
        let args = Args::parse();

        let formatter = Formatter3164 {
            facility: Facility::LOG_DAEMON,
            hostname: None,
            process: std::env::args().next().unwrap(),
            pid: 0,
        };

        let logger = match syslog::unix(formatter) {
            Err(e) => {
                eprintln!("impossible to connect to syslog: {:?}", e);
                return;
            },
            Ok(logger) => logger,
        };

        let level_filter = if args.verbose {
            LevelFilter::Debug
        } else {
            LevelFilter::Info
        };

        if let Err(e)
            = log::set_boxed_logger(Box::new(BasicLogger::new(logger)))
                .map(|()| log::set_max_level(level_filter))
        {
            eprintln!("impossible to set logger: {:?}", e);
            return;
        }

        let mut sigs = vec![SIGHUP];
        let mut signals = if let Ok(signals) = Signals::new(&sigs) {
            signals
        } else {
            error!("Failed to create signal handler");
            eprintln!("Failed to register signal handlers");
            return;
        };
        
        info!("Starting quic-speed-server {} with config: {}", env!("CARGO_PKG_VERSION"), args.config.display());

        let config = match Config::load(&args.config) {
            Ok(config) => config,
            Err(e) => {
                eprintln!("Error loading config: {:?}", e);
                return;
            }
        };
        info!("Config loaded");

        let plain_http = if let Ok(server) = server::PlainHttpServer::new(80, args.bind_device.as_deref().map(|s| s.as_bytes())) {
            server
        } else {
            eprintln!("Failed to initialize plain http server");
            return;
        };
        plain_http.start();

        std::thread::spawn(move || {
            for sig in signals.forever() {
                info!("Received signal: {:?}", sig);
                match sig {
                    SIGHUP => {
                        info!("Reloading config");
                        match Config::load(&args.config) {
                            Ok(config) => {
                                info!("Config reloaded");
                            },
                            Err(e) => {
                                eprintln!("Error reloading config: {:?}", e);
                            }
                        }
                    },
                    _ => {
                        warn!("Unhandled signal: {:?}", sig);
                    }
                }
            }
        });

        loop {
            std::thread::park();
        }
    }
}
