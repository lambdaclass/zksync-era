#!/usr/bin/env python3

import subprocess
from time import sleep

RUNNING_SERVER_LOG = "Running `target/release/zksync_server`" 
SERVER_STEP_CHECK = b'\xe2\x9c\x94'.decode("utf-8")

            
def filter_server_logs(server_log_pipe):
    for line in iter(server_log_pipe.readline, b''): # b'\n'-separated lines
        line = line.decode("utf-8").strip()
        if SERVER_STEP_CHECK in line:
            print("Server: ", line)
        if RUNNING_SERVER_LOG in line:
            sleep(60)
            print("-" * 5 + "Server is running" + "-" * 5)
            execute_example()
        if "INFO" in line:
            print("Server: ", line)

def filter_example_logs(example_log_pipe):
    for line in iter(example_log_pipe.readline, b''): # b'\n'-separated lines
        print("Example: ", line.decode("utf-8").strip())

def init_server():
    option = select_server()
    if option == 1:
        command_line_args = ["make", "demo_rollup_calldata",  "-C",  "../",]
    elif option == 2:
        command_line_args = ["make", "demo_rollup_blobs",  "-C",  "../",]
    elif option == 3:
        command_line_args = ["make", "demo_validium_calldata",  "-C",  "../"]
    elif option == 4:
        command_line_args = ["make", "demo_validium_blobs",  "-C",  "../"]
    else:
        print("Invalid option")
    
    print ("Building zksync server...")
    server = subprocess.Popen(command_line_args, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    with server.stdout: filter_server_logs(server.stdout)
    server.wait()
     

def execute_example():
    print("Running example...")
    command_line_args = ["cargo", "run", "--release",  "--bin",  "validium_mode_example"]
    example = subprocess.Popen(command_line_args, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    with example.stdout: filter_example_logs(example.stdout)
    example.wait()

def select_server():
    print("Which zksync server do you want to build?")
    print("1. Rollup + Calldata")
    print("2. Rollup + Blobs")
    print("3. Validium + Calldata")
    print("4. Validium + Blobs")
    return int(input("Enter your choice (1-4):")) 

def get_pubdata():
    print("Getting pubdata...")
def init_celestia_server():
    print("Initializing Celestia server...")
def submit_data_to_celestia():
    print("Submitting data to Celestia...")

def main():
    init_server()
    


if __name__ == '__main__':
    main()