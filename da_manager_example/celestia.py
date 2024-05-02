import subprocess
import socket
import time
import threading

# ANSI color escape codes
class colors:
    GREEN = '\033[92m'
    YELLOW = '\033[93m'
    MAGENTA = '\033[95m'
    ENDC = '\033[0m'  # Reset color

RUNNING_SERVER_LOG = "Running `target/release/zksync_server`"
SERVER_STEP_CHECK = b'\xe2\x9c\x94'.decode("utf-8")

def filter_server_logs(server_log_pipe, server_running_flag):
    for line in iter(server_log_pipe.readline, b''): # b'\n'-separated lines
        line = line.decode("utf-8").strip()
        if SERVER_STEP_CHECK in line:
            print(colors.GREEN + "Server: " + line + colors.ENDC)
        if RUNNING_SERVER_LOG in line:
            print("-" * 5 + colors.GREEN + " Server is running " + colors.ENDC + "-" * 5)
            time.sleep(20)
            server_running_flag.set()

def filter_example_logs(example_log_pipe):
    for line in iter(example_log_pipe.readline, b''): # b'\n'-separated lines
        line = line.decode("utf-8").strip()
        print(colors.MAGENTA + "Example: " + line + colors.ENDC)

def select_server():
    print("Which zksync server do you want to build?")
    print("1. Rollup + Calldata")
    print("2. Rollup + Blobs")
    print("3. Validium + Calldata")
    print("4. Validium + Blobs")
    option = int(input("Enter your choice (1-4): "))
    if option == 1:
        return ["make", "demo_rollup_calldata",  "-C",  "../",]
    elif option == 2:
        return ["make", "demo_rollup_blobs",  "-C",  "../",]
    elif option == 3:
        return ["make", "demo_validium_calldata",  "-C",  "../"]
    elif option == 4:
        return ["make", "demo_validium_blobs",  "-C",  "../"]
    else:
        print("Invalid option")
        return None

def run_server(server_command, server_running_flag):
    server_process = subprocess.Popen(server_command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    filter_server_logs(server_process.stdout, server_running_flag)

def run_example():
    command_line_args = ["cargo", "run", "--release", "--bin", "validium_mode_example"]
    example_process = subprocess.Popen(command_line_args, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    filter_example_logs(example_process.stdout)

def main():
    server_running_flag = threading.Event()
    server_command = select_server()
    if server_command:
        server_thread = threading.Thread(target=run_server, args=(server_command, server_running_flag))
        server_thread.start()
        server_running_flag.wait()  # Wait until the server is running
        run_example()
        server_thread.join()

if __name__ == "__main__":
    main()
