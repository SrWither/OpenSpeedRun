use std::os::unix::net::UnixStream;
use std::io::Write;
use std::env;
use std::process;

fn main() {
    let socket_path = "/tmp/openspeedrun.sock";
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <command>", args[0]);
        eprintln!("Commands: split, start, pause, reset");
        process::exit(1);
    }

    let cmd = args[1].trim();

    let valid_cmds = ["split", "start", "pause", "reset"];
    if !valid_cmds.contains(&cmd) {
        eprintln!("Invalid command '{}'", cmd);
        process::exit(1);
    }

    let mut stream = UnixStream::connect(socket_path).expect("Could not connect to the OpenSpeedRun socket");

    stream.write_all(cmd.as_bytes()).expect("Failed to write command");
    stream.write_all(b"\n").expect("Failed to write newline");

    stream.shutdown(std::net::Shutdown::Write).expect("Failed to shutdown write half");
}
