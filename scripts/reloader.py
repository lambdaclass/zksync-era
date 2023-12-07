import re 
import sys 
import os 

contracts = [
        'CONTRACTS_CREATE2_FACTORY_ADDR',
        'CONTRACTS_ADMIN_FACET_ADDR',
        'CONTRACTS_DIAMOND_UPGRADE_INIT_ADDR',
        'CONTRACTS_DEFAULT_UPGRADE_ADDR',
        'CONTRACTS_GOVERNANCE_ADDR',
        'CONTRACTS_MAILBOX_FACET_ADDR',
        'CONTRACTS_EXECUTOR_FACET_ADDR',
        'CONTRACTS_GETTERS_FACET_ADDR',
        'CONTRACTS_VERIFIER_ADDR',
        'CONTRACTS_DIAMOND_INIT_ADDR',
        'CONTRACTS_DIAMOND_PROXY_ADDR',
        'CONTRACTS_VALIDATOR_TIMELOCK_ADDR',
        'CONTRACTS_GENESIS_TX_HASH',
        'CONTRACTS_L1_ERC20_BRIDGE_PROXY_ADDR',
        'CONTRACTS_L1_ERC20_BRIDGE_IMPL_ADDR',
        'CONTRACTS_L1_WETH_BRIDGE_IMPL_ADDR',
        'CONTRACTS_L1_WETH_BRIDGE_PROXY_ADDR',
        'CONTRACTS_L1_ALLOW_LIST_ADDR',
        'CONTRACTS_L1_MULTICALL3_ADDR'
    ]


def modify(variable, assignedvariable):
    initEnv = "etc/env/.init.env"
    if not os.path.exists(initEnv):
        f = open(initEnv, "x")
        f.write(assignedvariable)
        return 
    source = open(initEnv).read()
    if assignedvariable in source:
        source.replace(variable, assignedvariable)


def reloadEnvs():
    envVars = []
    if sys.argv[1] == "contracts":
        envVars = contracts

    fd = open(sys.argv[2])
    file = fd.read()
    for env in file.split():
        envs = env.split("=")
        if envs[0] in envVars:
            modify(envs[0], env)

reloadEnvs()