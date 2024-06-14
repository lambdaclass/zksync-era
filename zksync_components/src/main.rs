use std::{
    io::{BufRead, BufReader, Write},
    process::{Command, Stdio},
    thread::{self, sleep},
    time::Duration,
};

fn store_logs(stdout: std::process::ChildStdout, file_name: &str) {
    let stdout_reader = BufReader::new(stdout);
    let mut file = std::fs::File::create(file_name).expect("Failed to create file");
    for line in stdout_reader.lines() {
        match line {
            Ok(line) => {
                file.write_all(line.as_bytes())
                    .expect("Failed to write to file");
                file.write_all(b"\n").expect("Failed to write to file");
            }
            Err(err) => eprintln!("Error reading stdout: {}", err),
        }
    }
    file.sync_all().expect("Failed to sync file");
    println!("Logs stored in file: {}", file_name);
}

fn run_component(zk_path: &str, component: &str, env_config: &str) {
    // Compile the environment configuration for the component
    let env_compile_command = Command::new(zk_path)
        .arg("config")
        .arg("compile")
        .arg(env_config)
        .output()
        .expect("Failed to execute zk config compile command");

    // Check if the environment compile command succeeded
    if !env_compile_command.status.success() {
        eprintln!(
            "Failed to compile environment configuration for {}",
            env_config
        );
        eprintln!(
            "stderr: {}",
            String::from_utf8_lossy(&env_compile_command.stderr)
        );
        return;
    }

    // Set the environment configuration for the component
    let env_command = Command::new(zk_path)
        .arg("env")
        .arg(env_config)
        .output()
        .expect("Failed to execute zk env command");

    // Check if the environment command succeeded
    if !env_command.status.success() {
        eprintln!("Failed to set environment configuration for {}", env_config);
        eprintln!("stderr: {}", String::from_utf8_lossy(&env_command.stderr));
        return;
    }

    // Spawn the command for the component
    let mut child = Command::new(zk_path)
        .arg("server")
        .arg(format!("--components={}", component))
        .stdout(Stdio::piped()) // Capture stdout
        .stderr(Stdio::piped()) // Capture stderr
        .spawn()
        .expect("Failed to spawn command");

    // Capture stdout
    let stdout = child.stdout.take().expect("Failed to capture stdout");
    let log_file_name = format!("{}.log", component);
    let stdout_thread = thread::spawn(move || {
        store_logs(stdout, &log_file_name);
    });

    // Capture stderr with component name prefix
    let stderr = child.stderr.take().expect("Failed to capture stderr");
    let component_name = component.to_string();
    let stderr_reader = BufReader::new(stderr);
    let stderr_thread = thread::spawn(move || {
        for line in stderr_reader.lines() {
            match line {
                Ok(line) => eprintln!("[{}] stderr: {}", component_name, line),
                Err(err) => eprintln!("[{}] Error reading stderr: {}", component_name, err),
            }
        }
    });

    // Wait for the child process to exit
    let status = child.wait().expect("Failed to wait on child");
    println!("Component '{}' exited with status: {}", component, status);

    // Wait for the threads to finish
    stdout_thread.join().expect("Failed to join stdout thread");
    stderr_thread.join().expect("Failed to join stderr thread");
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
        ("tree", "tree"),
        ("tree_api", "tree"),
    ];

    // Spawn threads for each component
    let mut handles = vec![];
    for (component, env_config) in components {
        let zk_path = zk_path.to_string();
        let handle = thread::spawn(move || {
            println!(
                "Running component '{}' with env '{}'",
                component, env_config
            );
            run_component(&zk_path, component, env_config);
            sleep(Duration::from_secs(1));
        });
        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().expect("Thread panicked");
    }
}
