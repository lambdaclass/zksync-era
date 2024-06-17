use std::{
    borrow::Borrow,
    io::{BufRead, BufReader, Write},
    process::{Command, Stdio},
    thread,
    time::Duration,
};

// Function to check ports used by a specific PID
fn check_ports(pid: u32) -> Vec<u16> {
    let output = Command::new("lsof")
        .arg("-p")
        .arg(pid.to_string())
        .arg("-i")
        .output()
        .expect("Failed to execute lsof command");

    let mut used_ports = Vec::new();

    if output.status.success() {
        println!("Checking ports used by PID: {}", pid);
        let output_str = String::from_utf8_lossy(&output.stdout);
        for line in output_str.lines() {
            if line.contains("TCP") || line.contains("UDP") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if let Some(port_part) = parts.get(8) {
                    if let Some(port_str) = port_part.split(':').last() {
                        if let Ok(port) = port_str.parse::<u16>() {
                            used_ports.push(port);
                        }
                    }
                }
            }
        }
    } else {
        eprintln!(
            "Failed to retrieve port information for PID {}: {}",
            pid,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    used_ports
}

// Function to store logs
fn store_logs<R: BufRead>(reader: R, file_name: &str) {
    let mut file = std::fs::File::create(file_name).expect("Failed to create file");
    for line in reader.lines() {
        match line {
            Ok(line) => {
                file.write_all(line.as_bytes())
                    .expect("Failed to write to file");
                file.write_all(b"\n").expect("Failed to write to file");
            }
            Err(err) => eprintln!("Error reading log: {}", err),
        }
    }
    file.sync_all().expect("Failed to sync file");
    println!("Logs stored in file: {}", file_name);
}

// Function to run components
fn run_component(zk_path: &str, component: &str, env_config: &str) {
    // Compile the environment configuration for the component
    let env_command = Command::new(zk_path)
        .arg("config")
        .arg("compile")
        .arg(env_config)
        .output()
        .expect("Failed to execute zk env command");

    if !env_command.status.success() {
        eprintln!("Failed to set environment configuration for {}", env_config);
        eprintln!("stderr: {}", String::from_utf8_lossy(&env_command.stderr));
        return;
    }

    println!("Config compiled for component {}", component);

    // Set the environment configuration for the component
    let env_command = Command::new(zk_path)
        .arg("env")
        .arg(env_config)
        .output()
        .expect("Failed to execute zk env command");

    if !env_command.status.success() {
        eprintln!("Failed to set environment configuration for {}", env_config);
        eprintln!("stderr: {}", String::from_utf8_lossy(&env_command.stderr));
        return;
    }

    println!("Environment set for component {}", component);

    // Spawn the component
    let mut child = Command::new(zk_path)
        .arg("server")
        .arg(format!("--components={}", component))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn command");

    let pid = child.id();

    println!("Component {} spawned with PID: {}", component, pid);

    // Thread to capture stdout and store logs
    let stdout = child.stdout.take().expect("Failed to capture stdout");
    let log_file_name = format!("{}.log", component);
    let stdout_thread = thread::spawn(move || {
        store_logs(BufReader::new(stdout), &log_file_name);
    });

    let stderr = child.stderr.take().expect("Failed to capture stderr");
    let err_file_name = format!("{}.err", component);
    let stderr_thread = thread::spawn(move || {
        store_logs(BufReader::new(stderr), &err_file_name);
    });

    // // Monitor ports used by the component
    // let monitor_thread = thread::spawn({
    //     let component_name = component.to_string();
    //     move || {
    //         for _ in 0..100 {
    //             let used_ports = check_ports(pid);
    //             if !used_ports.is_empty() {
    //                 println!("[{}] Ports in use: {:?}", component_name, used_ports);
    //             }
    //             thread::sleep(Duration::from_secs(1));
    //         }
    //     }
    // });

    // Wait for the child process to exit
    let status = child.wait().expect("Failed to wait on child");
    println!("Component {} exited with status: {}", component, status);

    // Wait for the threads to finish
    stdout_thread.join().expect("Failed to join stdout thread");
    stderr_thread.join().expect("Failed to join stderr thread");
    // monitor_thread.join().expect("Failed to join monitor thread");
}

fn main() {
    let zk_path = "../bin/zk";

    // Check if the binary exists
    if !std::path::Path::new(zk_path).exists() {
        eprintln!("zk binary not found at path: {}", zk_path);
        return;
    }

    // Define components and their corresponding env configurations
    let components = vec![
        ("http_api", "http_api"),
        ("ws_api", "ws_api"),
        ("contract_verification_api", "contract_verification_api"),
        ("state_keeper", "state_keeper"),
        ("housekeeper", "housekeeper"),
        ("tee_verifier_input_producer", "tee_verifier_input_producer"),
        ("eth_watcher", "eth_watcher"),
        ("eth_tx_aggregator", "eth_tx_aggregator"),
        ("eth_tx_manager", "eth_tx_manager"),
        ("proof_data_handler", "proof_data_handler"),
        ("consensus", "consensus"),
        ("commitment_generator", "commitment_generator"),
        ("tree,tree_api", "tree"),
    ];

    let mut handles = vec![];

    for (component, env_config) in components {
        println!("Running component: {}", component);
        let handle = thread::spawn(move || {
            run_component(&zk_path, &component, &env_config);
        });
        handles.push(handle);
        thread::sleep(Duration::from_secs(5)); // Wait for 5 seconds before starting the next component
    }

    for handle in handles {
        handle.join().expect("Failed to join thread");
    }
}
