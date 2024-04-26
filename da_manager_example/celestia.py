#!/usr/bin/env python3

import logging
from subprocess import Popen, PIPE, STDOUT
import multiprocessing as mp

def log_subprocess_output(pipe, process_name, starts_with, ends_with, matches):
    for line in iter(pipe.readline, b''): # b'\n'-separated lines
        line = line.decode("utf-8").strip()
        if (starts_with != None and line.startswith(starts_with)) or any(line for match in matches if match in line):
            print(process_name + ": ", line)
        if ends_with != None and ends_with in line:
            print("-" * 5 + process_name + " executing." + "-" * 5)
            if process_name == "Server": 
                mp.Process(target=execute_example).start()
            else:
                print("Process ended")
            
        
def init_server():
    # print("Which zksync server do you want to build?")
    # print("1. Rollup + Calldata")
    # print("2. Rollup + Blobs")
    # print("3. Validium + Calldata")
    # print("4. Validium + Blobs")
    # option = int(input("Enter your choice (1-4): "))  
    # if option == 1:
    #     process = subprocess.run(["make", "demo_rollup_calldata",  "-C",  "../",])
    # elif option == 2:
    #     process = subprocess.run(["make", "demo_rollup_blobs",  "-C",  "../",])
    # elif option == 3:
    #     process = subprocess.run(["make", "demo_validium_calldata",  "-C",  "../"])
    # elif option == 4:
    #     process = subprocess.run(["make", "demo_validium_blobs",  "-C",  "../"])
    # else:
    #     print("Invalid option")
    command_line_args = ["make", "demo_validium_calldata",  "-C",  "../"]
    process = Popen(command_line_args, stdout=PIPE, stderr=STDOUT)
    with process.stdout:
        server_started = "Running `target/release/zksync_server`"
        check = b'\xe2\x9c\x94'.decode("utf-8")
        log_subprocess_output(process.stdout, "Server", ">", server_started, [check])
    return process.wait() # 0 means success
     

def execute_example():

    command_line_args = ["cargo", "run", "--release",  "--bin",  "validium_mode_example"]
    process = Popen(command_line_args, stdout=PIPE, stderr=STDOUT)
    with process.stdout:
        log_subprocess_output(process.stdout, "Example", "Running", None, matches = ["Deposit", "Mint", "Deploy", "Transfer"])
    return process.wait() # 0 means succes

def main():
    process_list = []
    process_list.append(mp.Process(target=init_server))
    for p in process_list:
        p.start()

if __name__ == '__main__':
    main()