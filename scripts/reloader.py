import re 
import sys 
import os 

INIT_ENV = "etc/env/.init.env"
ENV = ".env"

# ----------------------------------------
#       variables to update
# ----------------------------------------

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

genesis = [
    'CONTRACTS_GENESIS_BATCH_COMMITMENT',
    'CHAIN_STATE_KEEPER_BOOTLOADER_HASH',
    'CHAIN_STATE_KEEPER_DEFAULT_AA_HASH',
    'CONTRACTS_GENESIS_ROOT',
    'CONTRACTS_GENESIS_ROLLUP_LEAF_INDEX'
]

L2deploy = [
    'CONTRACTS_L2_ERC20_BRIDGE_ADDR',
    'CONTRACTS_L2_TESTNET_PAYMASTER_ADDR',
    'CONTRACTS_L2_WETH_TOKEN_IMPL_ADDR',
    'CONTRACTS_L2_WETH_TOKEN_PROXY_ADDR',
    'CONTRACTS_L2_DEFAULT_UPGRADE_ADDR'
]

L1deploy = [
    'CONTRACTS_L2_WETH_BRIDGE_ADDR'
]

# ----------------------------------------
#            Function calls
# ----------------------------------------


def open_as_dict(path):
    file = open(path).read()
    variables = file.split()
    env_vars = {}
    for var in variables:
        env = var.split("=")
        if len(env) > 1:
            env_vars[env[0]] = env[1]
    return env_vars

def update_dotenv(init_envs):
    filenv = open(ENV, "w")
    dotenv = open_as_dict(ENV)
    for k, v in init_envs.items():
        dotenv[k] = v
    for k, v in dotenv.items():
        val = "{key}={value}\n".format(key=k,value=v)
        filenv.write(val)
    filenv.close()

def reload(path, init_envs):   
    file = open(path, "w")
    for k, v in init_envs.items():
        val = "{key}={value}\n".format(key=k,value=v)
        file.write(val)
    file.close()
    update_dotenv(init_envs)
    
def modify(envs):
    init_envs = open_as_dict(INIT_ENV)
    for k, v in envs.items():
        init_envs[k] = v
    reload(INIT_ENV, init_envs)


def reload_envs():
    env_vars = []
    if sys.argv[1] == "contracts":
        env_vars = contracts
    elif sys.argv[1] == "genesis":
        env_vars = genesis
    elif sys.argv[1] == "L2deploy":
        env_vars = L2deploy
    elif sys.argv[1] == "L1deploy":
        env_vars = L1deploy

    # read logs
    envs = open_as_dict(sys.argv[2])
    # assigned them
    modify(envs)

reload_envs()