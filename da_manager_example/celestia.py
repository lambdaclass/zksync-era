import subprocess



def main():
    print("Hello, Celestia!")
    subprocess.run(["celestia", "light", "init", "--p2p.network", "arabica"])
    subprocess.run(["celestia", "light",  "start",  "--core.ip",  " /Users/toni-calvin/.celestia-light-arabica-11/", "--p2p.network",  "arabica"]) 


if __name__ == '__main__':
    main()