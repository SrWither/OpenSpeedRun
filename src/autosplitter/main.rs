//! Polls a target (a RetroArch-compatible emulator, or opt-in a native
//! process's memory) and turns configured value transitions into
//! `openspeedrun` control commands, sent over the same control socket
//! `openspeedrun-cli` uses. See `openspeedrun::autosplitter` for the
//! security reasoning behind why native-process reading is opt-in.

use std::env;
use std::process;

#[cfg(unix)]
fn main() {
    use std::collections::HashMap;
    use std::io::Write;
    use std::os::unix::net::UnixStream;
    use std::thread;
    use std::time::Duration;

    #[cfg(target_os = "linux")]
    use openspeedrun::autosplitter::config::Watch;
    use openspeedrun::autosplitter::config::{AutosplitterConfig, Target};
    // `process_memory`'s process-reading API only compiles on Linux (it's
    // built on `/proc`, which doesn't exist on macOS or *BSD) — see that
    // module's docs. Everything importing from it below is only used inside
    // the `Target::ProcessMemory` arm, which is itself Linux-only for the
    // same reason.
    #[cfg(target_os = "linux")]
    use openspeedrun::autosplitter::process_memory::{
        ProcessMemoryReader, find_module_base, find_pid_by_name, resolve_pointer_chain,
    };
    use openspeedrun::autosplitter::retroarch::RetroArchClient;
    use openspeedrun::core::socket_path;

    let args: Vec<String> = env::args().collect();
    let Some(config_path) = args.get(1) else {
        eprintln!("Usage: {} <autosplitter.json>", args[0]);
        process::exit(1);
    };

    let config = match AutosplitterConfig::load_from_file(config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to load autosplitter config: {e}");
            process::exit(1);
        }
    };

    let poll_interval = Duration::from_millis(config.poll_interval_ms);
    let mut previous: HashMap<String, i128> = HashMap::new();

    let send_command = |cmd: &str, watch_name: &str| match UnixStream::connect(socket_path()) {
        Ok(mut stream) => {
            if let Err(e) = writeln!(stream, "{cmd}") {
                eprintln!("Failed to send '{cmd}' for watch '{watch_name}': {e}");
            } else {
                println!("Watch '{watch_name}' triggered -> sent '{cmd}'");
            }
        }
        Err(e) => eprintln!("Failed to connect to openspeedrun socket: {e}"),
    };

    match &config.target {
        Target::Retroarch { host, port } => {
            let client = match RetroArchClient::connect(host, *port) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Failed to connect to RetroArch at {host}:{port}: {e}");
                    process::exit(1);
                }
            };
            println!(
                "Connected to RetroArch at {host}:{port}, watching {} value(s)",
                config.watches.len()
            );

            loop {
                for watch in &config.watches {
                    let address = match watch.address() {
                        Ok(a) => a,
                        Err(e) => {
                            eprintln!("Skipping watch '{}': {e}", watch.name);
                            continue;
                        }
                    };

                    let bytes = match client.read_memory(address, watch.value_type.size()) {
                        Ok(b) => b,
                        Err(e) => {
                            eprintln!("Read failed for watch '{}': {e}", watch.name);
                            continue;
                        }
                    };

                    let Some(current) = watch.value_type.decode(&bytes, watch.endian) else {
                        eprintln!("Short read for watch '{}'", watch.name);
                        continue;
                    };

                    let prev = previous.get(&watch.name).copied();
                    if watch.condition.triggered(prev, current) {
                        send_command(watch.action.as_command(), &watch.name);
                    }
                    previous.insert(watch.name.clone(), current);
                }

                thread::sleep(poll_interval);
            }
        }

        #[cfg(not(target_os = "linux"))]
        Target::ProcessMemory { .. } => {
            eprintln!(
                "The 'process_memory' target needs /proc, which only exists on Linux \
                 (not macOS or *BSD). Use the 'retroarch' target instead, or run this \
                 on Linux."
            );
            process::exit(1);
        }

        #[cfg(target_os = "linux")]
        Target::ProcessMemory { process_name } => {
            // Resolves the address a watch should ultimately read: `module`
            // (if set) plus `address`, then chases `pointer_path` through
            // `reader`. Module bases are cached per-attach in `module_bases`
            // since they don't change for the lifetime of a process.
            fn resolve_watch_address(
                watch: &Watch,
                reader: &ProcessMemoryReader,
                maps_text: &str,
                module_bases: &mut HashMap<String, u64>,
            ) -> Result<u64, String> {
                let offset = watch.address()?;
                let base = match &watch.module {
                    Some(module) => {
                        if let Some(&base) = module_bases.get(module) {
                            base
                        } else {
                            let base = find_module_base(maps_text, module).ok_or_else(|| {
                                format!("module '{module}' not found in the process's memory map")
                            })?;
                            module_bases.insert(module.clone(), base);
                            base
                        }
                    }
                    None => 0,
                };

                let pointer_path = watch.pointer_path_values()?;
                resolve_pointer_chain(
                    |addr| reader.read_u64(addr).ok(),
                    base + offset,
                    &pointer_path,
                )
                .ok_or_else(|| {
                    "pointer chase failed (a dereference read didn't land in mapped memory)"
                        .to_string()
                })
            }

            'attach: loop {
                println!("Looking for a process named '{process_name}'...");
                let pid = loop {
                    match find_pid_by_name(process_name) {
                        Ok(Some(pid)) => break pid,
                        Ok(None) => thread::sleep(poll_interval),
                        Err(e) => {
                            eprintln!("Failed to scan /proc: {e}");
                            process::exit(1);
                        }
                    }
                };

                let reader = match ProcessMemoryReader::open(pid) {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!("{e}");
                        process::exit(1);
                    }
                };
                println!(
                    "Attached to '{process_name}' (pid {pid}), watching {} value(s)",
                    config.watches.len()
                );

                let mut module_bases: HashMap<String, u64> = HashMap::new();

                loop {
                    let maps_text = match std::fs::read_to_string(format!("/proc/{pid}/maps")) {
                        Ok(t) => t,
                        Err(_) => {
                            println!(
                                "Process {pid} appears to have exited; waiting for it to restart..."
                            );
                            previous.clear();
                            continue 'attach;
                        }
                    };

                    for watch in &config.watches {
                        let address = match resolve_watch_address(
                            watch,
                            &reader,
                            &maps_text,
                            &mut module_bases,
                        ) {
                            Ok(a) => a,
                            Err(e) => {
                                eprintln!("Skipping watch '{}': {e}", watch.name);
                                continue;
                            }
                        };

                        let bytes = match reader.read_at(address, watch.value_type.size()) {
                            Ok(b) => b,
                            Err(e) => {
                                eprintln!("Read failed for watch '{}': {e}", watch.name);
                                continue;
                            }
                        };

                        let Some(current) = watch.value_type.decode(&bytes, watch.endian) else {
                            eprintln!("Short read for watch '{}'", watch.name);
                            continue;
                        };

                        let prev = previous.get(&watch.name).copied();
                        if watch.condition.triggered(prev, current) {
                            send_command(watch.action.as_command(), &watch.name);
                        }
                        previous.insert(watch.name.clone(), current);
                    }

                    thread::sleep(poll_interval);
                }
            }
        }
    }
}

#[cfg(windows)]
fn main() {
    eprintln!("openspeedrun-autosplitter is Unix-only (uses the Unix control socket).");
    process::exit(1);
}
