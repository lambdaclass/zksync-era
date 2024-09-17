const { Web3 } = require('web3');
const web3 = new Web3('http://localhost:8545');

const abi = [
  {
    "inputs": [
      {
        "internalType": "address",
        "name": "_initialOwner",
        "type": "address"
      },
      {
        "internalType": "uint32",
        "name": "_executionDelay",
        "type": "uint32"
      },
      {
        "internalType": "uint256",
        "name": "_eraChainId",
        "type": "uint256"
      }
    ],
    "stateMutability": "nonpayable",
    "type": "constructor"
  },
  {
    "inputs": [
      {
        "internalType": "uint256",
        "name": "_chainId",
        "type": "uint256"
      }
    ],
    "name": "AddressAlreadyValidator",
    "type": "error"
  },
  {
    "inputs": [
      {
        "internalType": "uint256",
        "name": "_chainId",
        "type": "uint256"
      }
    ],
    "name": "ValidatorDoesNotExist",
    "type": "error"
  },
  {
    "anonymous": false,
    "inputs": [
      {
        "indexed": true,
        "internalType": "uint256",
        "name": "batchNumber",
        "type": "uint256"
      },
      {
        "indexed": true,
        "internalType": "bytes32",
        "name": "batchHash",
        "type": "bytes32"
      },
      {
        "indexed": true,
        "internalType": "bytes32",
        "name": "commitment",
        "type": "bytes32"
      }
    ],
    "name": "BlockCommit",
    "type": "event"
  },
  {
    "anonymous": false,
    "inputs": [
      {
        "indexed": true,
        "internalType": "uint256",
        "name": "batchNumber",
        "type": "uint256"
      },
      {
        "indexed": true,
        "internalType": "bytes32",
        "name": "batchHash",
        "type": "bytes32"
      },
      {
        "indexed": true,
        "internalType": "bytes32",
        "name": "commitment",
        "type": "bytes32"
      }
    ],
    "name": "BlockExecution",
    "type": "event"
  },
  {
    "anonymous": false,
    "inputs": [
      {
        "indexed": false,
        "internalType": "uint256",
        "name": "totalBatchesCommitted",
        "type": "uint256"
      },
      {
        "indexed": false,
        "internalType": "uint256",
        "name": "totalBatchesVerified",
        "type": "uint256"
      },
      {
        "indexed": false,
        "internalType": "uint256",
        "name": "totalBatchesExecuted",
        "type": "uint256"
      }
    ],
    "name": "BlocksRevert",
    "type": "event"
  },
  {
    "anonymous": false,
    "inputs": [
      {
        "indexed": true,
        "internalType": "uint256",
        "name": "previousLastVerifiedBatch",
        "type": "uint256"
      },
      {
        "indexed": true,
        "internalType": "uint256",
        "name": "currentLastVerifiedBatch",
        "type": "uint256"
      }
    ],
    "name": "BlocksVerification",
    "type": "event"
  },
  {
    "anonymous": false,
    "inputs": [
      {
        "indexed": false,
        "internalType": "uint256",
        "name": "_newExecutionDelay",
        "type": "uint256"
      }
    ],
    "name": "NewExecutionDelay",
    "type": "event"
  },
  {
    "anonymous": false,
    "inputs": [
      {
        "indexed": true,
        "internalType": "address",
        "name": "previousOwner",
        "type": "address"
      },
      {
        "indexed": true,
        "internalType": "address",
        "name": "newOwner",
        "type": "address"
      }
    ],
    "name": "OwnershipTransferStarted",
    "type": "event"
  },
  {
    "anonymous": false,
    "inputs": [
      {
        "indexed": true,
        "internalType": "address",
        "name": "previousOwner",
        "type": "address"
      },
      {
        "indexed": true,
        "internalType": "address",
        "name": "newOwner",
        "type": "address"
      }
    ],
    "name": "OwnershipTransferred",
    "type": "event"
  },
  {
    "anonymous": false,
    "inputs": [
      {
        "indexed": true,
        "internalType": "uint256",
        "name": "_chainId",
        "type": "uint256"
      },
      {
        "indexed": false,
        "internalType": "address",
        "name": "_addedValidator",
        "type": "address"
      }
    ],
    "name": "ValidatorAdded",
    "type": "event"
  },
  {
    "anonymous": false,
    "inputs": [
      {
        "indexed": true,
        "internalType": "uint256",
        "name": "_chainId",
        "type": "uint256"
      },
      {
        "indexed": false,
        "internalType": "address",
        "name": "_removedValidator",
        "type": "address"
      }
    ],
    "name": "ValidatorRemoved",
    "type": "event"
  },
  {
    "inputs": [],
    "name": "acceptOwnership",
    "outputs": [],
    "stateMutability": "nonpayable",
    "type": "function"
  },
  {
    "inputs": [
      {
        "internalType": "uint256",
        "name": "_chainId",
        "type": "uint256"
      },
      {
        "internalType": "address",
        "name": "_newValidator",
        "type": "address"
      }
    ],
    "name": "addValidator",
    "outputs": [],
    "stateMutability": "nonpayable",
    "type": "function"
  },
  {
    "inputs": [
      {
        "components": [
          {
            "internalType": "uint64",
            "name": "batchNumber",
            "type": "uint64"
          },
          {
            "internalType": "bytes32",
            "name": "batchHash",
            "type": "bytes32"
          },
          {
            "internalType": "uint64",
            "name": "indexRepeatedStorageChanges",
            "type": "uint64"
          },
          {
            "internalType": "uint256",
            "name": "numberOfLayer1Txs",
            "type": "uint256"
          },
          {
            "internalType": "bytes32",
            "name": "priorityOperationsHash",
            "type": "bytes32"
          },
          {
            "internalType": "bytes32",
            "name": "l2LogsTreeRoot",
            "type": "bytes32"
          },
          {
            "internalType": "uint256",
            "name": "timestamp",
            "type": "uint256"
          },
          {
            "internalType": "bytes32",
            "name": "commitment",
            "type": "bytes32"
          }
        ],
        "internalType": "struct IExecutor.StoredBatchInfo",
        "name": "",
        "type": "tuple"
      },
      {
        "components": [
          {
            "internalType": "uint64",
            "name": "batchNumber",
            "type": "uint64"
          },
          {
            "internalType": "uint64",
            "name": "timestamp",
            "type": "uint64"
          },
          {
            "internalType": "uint64",
            "name": "indexRepeatedStorageChanges",
            "type": "uint64"
          },
          {
            "internalType": "bytes32",
            "name": "newStateRoot",
            "type": "bytes32"
          },
          {
            "internalType": "uint256",
            "name": "numberOfLayer1Txs",
            "type": "uint256"
          },
          {
            "internalType": "bytes32",
            "name": "priorityOperationsHash",
            "type": "bytes32"
          },
          {
            "internalType": "bytes32",
            "name": "bootloaderHeapInitialContentsHash",
            "type": "bytes32"
          },
          {
            "internalType": "bytes32",
            "name": "eventsQueueStateHash",
            "type": "bytes32"
          },
          {
            "internalType": "bytes",
            "name": "systemLogs",
            "type": "bytes"
          },
          {
            "internalType": "bytes",
            "name": "pubdataCommitments",
            "type": "bytes"
          }
        ],
        "internalType": "struct IExecutor.CommitBatchInfo[]",
        "name": "_newBatchesData",
        "type": "tuple[]"
      }
    ],
    "name": "commitBatches",
    "outputs": [],
    "stateMutability": "nonpayable",
    "type": "function"
  },
  {
    "inputs": [
      {
        "internalType": "uint256",
        "name": "_chainId",
        "type": "uint256"
      },
      {
        "components": [
          {
            "internalType": "uint64",
            "name": "batchNumber",
            "type": "uint64"
          },
          {
            "internalType": "bytes32",
            "name": "batchHash",
            "type": "bytes32"
          },
          {
            "internalType": "uint64",
            "name": "indexRepeatedStorageChanges",
            "type": "uint64"
          },
          {
            "internalType": "uint256",
            "name": "numberOfLayer1Txs",
            "type": "uint256"
          },
          {
            "internalType": "bytes32",
            "name": "priorityOperationsHash",
            "type": "bytes32"
          },
          {
            "internalType": "bytes32",
            "name": "l2LogsTreeRoot",
            "type": "bytes32"
          },
          {
            "internalType": "uint256",
            "name": "timestamp",
            "type": "uint256"
          },
          {
            "internalType": "bytes32",
            "name": "commitment",
            "type": "bytes32"
          }
        ],
        "internalType": "struct IExecutor.StoredBatchInfo",
        "name": "",
        "type": "tuple"
      },
      {
        "components": [
          {
            "internalType": "uint64",
            "name": "batchNumber",
            "type": "uint64"
          },
          {
            "internalType": "uint64",
            "name": "timestamp",
            "type": "uint64"
          },
          {
            "internalType": "uint64",
            "name": "indexRepeatedStorageChanges",
            "type": "uint64"
          },
          {
            "internalType": "bytes32",
            "name": "newStateRoot",
            "type": "bytes32"
          },
          {
            "internalType": "uint256",
            "name": "numberOfLayer1Txs",
            "type": "uint256"
          },
          {
            "internalType": "bytes32",
            "name": "priorityOperationsHash",
            "type": "bytes32"
          },
          {
            "internalType": "bytes32",
            "name": "bootloaderHeapInitialContentsHash",
            "type": "bytes32"
          },
          {
            "internalType": "bytes32",
            "name": "eventsQueueStateHash",
            "type": "bytes32"
          },
          {
            "internalType": "bytes",
            "name": "systemLogs",
            "type": "bytes"
          },
          {
            "internalType": "bytes",
            "name": "pubdataCommitments",
            "type": "bytes"
          }
        ],
        "internalType": "struct IExecutor.CommitBatchInfo[]",
        "name": "_newBatchesData",
        "type": "tuple[]"
      }
    ],
    "name": "commitBatchesSharedBridge",
    "outputs": [],
    "stateMutability": "nonpayable",
    "type": "function"
  },
  {
    "inputs": [
      {
        "components": [
          {
            "internalType": "uint64",
            "name": "batchNumber",
            "type": "uint64"
          },
          {
            "internalType": "bytes32",
            "name": "batchHash",
            "type": "bytes32"
          },
          {
            "internalType": "uint64",
            "name": "indexRepeatedStorageChanges",
            "type": "uint64"
          },
          {
            "internalType": "uint256",
            "name": "numberOfLayer1Txs",
            "type": "uint256"
          },
          {
            "internalType": "bytes32",
            "name": "priorityOperationsHash",
            "type": "bytes32"
          },
          {
            "internalType": "bytes32",
            "name": "l2LogsTreeRoot",
            "type": "bytes32"
          },
          {
            "internalType": "uint256",
            "name": "timestamp",
            "type": "uint256"
          },
          {
            "internalType": "bytes32",
            "name": "commitment",
            "type": "bytes32"
          }
        ],
        "internalType": "struct IExecutor.StoredBatchInfo[]",
        "name": "_newBatchesData",
        "type": "tuple[]"
      }
    ],
    "name": "executeBatches",
    "outputs": [],
    "stateMutability": "nonpayable",
    "type": "function"
  },
  {
    "inputs": [
      {
        "internalType": "uint256",
        "name": "_chainId",
        "type": "uint256"
      },
      {
        "components": [
          {
            "internalType": "uint64",
            "name": "batchNumber",
            "type": "uint64"
          },
          {
            "internalType": "bytes32",
            "name": "batchHash",
            "type": "bytes32"
          },
          {
            "internalType": "uint64",
            "name": "indexRepeatedStorageChanges",
            "type": "uint64"
          },
          {
            "internalType": "uint256",
            "name": "numberOfLayer1Txs",
            "type": "uint256"
          },
          {
            "internalType": "bytes32",
            "name": "priorityOperationsHash",
            "type": "bytes32"
          },
          {
            "internalType": "bytes32",
            "name": "l2LogsTreeRoot",
            "type": "bytes32"
          },
          {
            "internalType": "uint256",
            "name": "timestamp",
            "type": "uint256"
          },
          {
            "internalType": "bytes32",
            "name": "commitment",
            "type": "bytes32"
          }
        ],
        "internalType": "struct IExecutor.StoredBatchInfo[]",
        "name": "_newBatchesData",
        "type": "tuple[]"
      }
    ],
    "name": "executeBatchesSharedBridge",
    "outputs": [],
    "stateMutability": "nonpayable",
    "type": "function"
  },
  {
    "inputs": [],
    "name": "executionDelay",
    "outputs": [
      {
        "internalType": "uint32",
        "name": "",
        "type": "uint32"
      }
    ],
    "stateMutability": "view",
    "type": "function"
  },
  {
    "inputs": [
      {
        "internalType": "uint256",
        "name": "_chainId",
        "type": "uint256"
      },
      {
        "internalType": "uint256",
        "name": "_l2BatchNumber",
        "type": "uint256"
      }
    ],
    "name": "getCommittedBatchTimestamp",
    "outputs": [
      {
        "internalType": "uint256",
        "name": "",
        "type": "uint256"
      }
    ],
    "stateMutability": "view",
    "type": "function"
  },
  {
    "inputs": [],
    "name": "getName",
    "outputs": [
      {
        "internalType": "string",
        "name": "",
        "type": "string"
      }
    ],
    "stateMutability": "view",
    "type": "function"
  },
  {
    "inputs": [],
    "name": "owner",
    "outputs": [
      {
        "internalType": "address",
        "name": "",
        "type": "address"
      }
    ],
    "stateMutability": "view",
    "type": "function"
  },
  {
    "inputs": [],
    "name": "pendingOwner",
    "outputs": [
      {
        "internalType": "address",
        "name": "",
        "type": "address"
      }
    ],
    "stateMutability": "view",
    "type": "function"
  },
  {
    "inputs": [
      {
        "components": [
          {
            "internalType": "uint64",
            "name": "batchNumber",
            "type": "uint64"
          },
          {
            "internalType": "bytes32",
            "name": "batchHash",
            "type": "bytes32"
          },
          {
            "internalType": "uint64",
            "name": "indexRepeatedStorageChanges",
            "type": "uint64"
          },
          {
            "internalType": "uint256",
            "name": "numberOfLayer1Txs",
            "type": "uint256"
          },
          {
            "internalType": "bytes32",
            "name": "priorityOperationsHash",
            "type": "bytes32"
          },
          {
            "internalType": "bytes32",
            "name": "l2LogsTreeRoot",
            "type": "bytes32"
          },
          {
            "internalType": "uint256",
            "name": "timestamp",
            "type": "uint256"
          },
          {
            "internalType": "bytes32",
            "name": "commitment",
            "type": "bytes32"
          }
        ],
        "internalType": "struct IExecutor.StoredBatchInfo",
        "name": "",
        "type": "tuple"
      },
      {
        "components": [
          {
            "internalType": "uint64",
            "name": "batchNumber",
            "type": "uint64"
          },
          {
            "internalType": "bytes32",
            "name": "batchHash",
            "type": "bytes32"
          },
          {
            "internalType": "uint64",
            "name": "indexRepeatedStorageChanges",
            "type": "uint64"
          },
          {
            "internalType": "uint256",
            "name": "numberOfLayer1Txs",
            "type": "uint256"
          },
          {
            "internalType": "bytes32",
            "name": "priorityOperationsHash",
            "type": "bytes32"
          },
          {
            "internalType": "bytes32",
            "name": "l2LogsTreeRoot",
            "type": "bytes32"
          },
          {
            "internalType": "uint256",
            "name": "timestamp",
            "type": "uint256"
          },
          {
            "internalType": "bytes32",
            "name": "commitment",
            "type": "bytes32"
          }
        ],
        "internalType": "struct IExecutor.StoredBatchInfo[]",
        "name": "",
        "type": "tuple[]"
      },
      {
        "components": [
          {
            "internalType": "uint256[]",
            "name": "recursiveAggregationInput",
            "type": "uint256[]"
          },
          {
            "internalType": "uint256[]",
            "name": "serializedProof",
            "type": "uint256[]"
          }
        ],
        "internalType": "struct IExecutor.ProofInput",
        "name": "",
        "type": "tuple"
      }
    ],
    "name": "proveBatches",
    "outputs": [],
    "stateMutability": "nonpayable",
    "type": "function"
  },
  {
    "inputs": [
      {
        "internalType": "uint256",
        "name": "_chainId",
        "type": "uint256"
      },
      {
        "components": [
          {
            "internalType": "uint64",
            "name": "batchNumber",
            "type": "uint64"
          },
          {
            "internalType": "bytes32",
            "name": "batchHash",
            "type": "bytes32"
          },
          {
            "internalType": "uint64",
            "name": "indexRepeatedStorageChanges",
            "type": "uint64"
          },
          {
            "internalType": "uint256",
            "name": "numberOfLayer1Txs",
            "type": "uint256"
          },
          {
            "internalType": "bytes32",
            "name": "priorityOperationsHash",
            "type": "bytes32"
          },
          {
            "internalType": "bytes32",
            "name": "l2LogsTreeRoot",
            "type": "bytes32"
          },
          {
            "internalType": "uint256",
            "name": "timestamp",
            "type": "uint256"
          },
          {
            "internalType": "bytes32",
            "name": "commitment",
            "type": "bytes32"
          }
        ],
        "internalType": "struct IExecutor.StoredBatchInfo",
        "name": "",
        "type": "tuple"
      },
      {
        "components": [
          {
            "internalType": "uint64",
            "name": "batchNumber",
            "type": "uint64"
          },
          {
            "internalType": "bytes32",
            "name": "batchHash",
            "type": "bytes32"
          },
          {
            "internalType": "uint64",
            "name": "indexRepeatedStorageChanges",
            "type": "uint64"
          },
          {
            "internalType": "uint256",
            "name": "numberOfLayer1Txs",
            "type": "uint256"
          },
          {
            "internalType": "bytes32",
            "name": "priorityOperationsHash",
            "type": "bytes32"
          },
          {
            "internalType": "bytes32",
            "name": "l2LogsTreeRoot",
            "type": "bytes32"
          },
          {
            "internalType": "uint256",
            "name": "timestamp",
            "type": "uint256"
          },
          {
            "internalType": "bytes32",
            "name": "commitment",
            "type": "bytes32"
          }
        ],
        "internalType": "struct IExecutor.StoredBatchInfo[]",
        "name": "",
        "type": "tuple[]"
      },
      {
        "components": [
          {
            "internalType": "uint256[]",
            "name": "recursiveAggregationInput",
            "type": "uint256[]"
          },
          {
            "internalType": "uint256[]",
            "name": "serializedProof",
            "type": "uint256[]"
          }
        ],
        "internalType": "struct IExecutor.ProofInput",
        "name": "",
        "type": "tuple"
      }
    ],
    "name": "proveBatchesSharedBridge",
    "outputs": [],
    "stateMutability": "nonpayable",
    "type": "function"
  },
  {
    "inputs": [
      {
        "internalType": "uint256",
        "name": "_chainId",
        "type": "uint256"
      },
      {
        "internalType": "address",
        "name": "_validator",
        "type": "address"
      }
    ],
    "name": "removeValidator",
    "outputs": [],
    "stateMutability": "nonpayable",
    "type": "function"
  },
  {
    "inputs": [],
    "name": "renounceOwnership",
    "outputs": [],
    "stateMutability": "nonpayable",
    "type": "function"
  },
  {
    "inputs": [
      {
        "internalType": "uint256",
        "name": "",
        "type": "uint256"
      }
    ],
    "name": "revertBatches",
    "outputs": [],
    "stateMutability": "nonpayable",
    "type": "function"
  },
  {
    "inputs": [
      {
        "internalType": "uint256",
        "name": "_chainId",
        "type": "uint256"
      },
      {
        "internalType": "uint256",
        "name": "",
        "type": "uint256"
      }
    ],
    "name": "revertBatchesSharedBridge",
    "outputs": [],
    "stateMutability": "nonpayable",
    "type": "function"
  },
  {
    "inputs": [
      {
        "internalType": "uint32",
        "name": "_executionDelay",
        "type": "uint32"
      }
    ],
    "name": "setExecutionDelay",
    "outputs": [],
    "stateMutability": "nonpayable",
    "type": "function"
  },
  {
    "inputs": [
      {
        "internalType": "contract IStateTransitionManager",
        "name": "_stateTransitionManager",
        "type": "address"
      }
    ],
    "name": "setStateTransitionManager",
    "outputs": [],
    "stateMutability": "nonpayable",
    "type": "function"
  },
  {
    "inputs": [],
    "name": "stateTransitionManager",
    "outputs": [
      {
        "internalType": "contract IStateTransitionManager",
        "name": "",
        "type": "address"
      }
    ],
    "stateMutability": "view",
    "type": "function"
  },
  {
    "inputs": [
      {
        "internalType": "address",
        "name": "newOwner",
        "type": "address"
      }
    ],
    "name": "transferOwnership",
    "outputs": [],
    "stateMutability": "nonpayable",
    "type": "function"
  },
  {
    "inputs": [
      {
        "internalType": "uint256",
        "name": "_chainId",
        "type": "uint256"
      },
      {
        "internalType": "address",
        "name": "_validator",
        "type": "address"
      }
    ],
    "name": "validators",
    "outputs": [
      {
        "internalType": "bool",
        "name": "",
        "type": "bool"
      }
    ],
    "stateMutability": "view",
    "type": "function"
  }
]

const contract = new web3.eth.Contract(abi);

function hexToUtf8(hex) {
  // Remove the '0x' prefix if present
  if (hex.startsWith('0x')) {
    hex = hex.slice(2);
  }

  // Ensure the hex string has an even length
  if (hex.length % 2 !== 0) {
    throw new Error('Invalid hex string length');
  }

  // Convert hex string to a byte array
  const bytes = [];
  for (let i = 0; i < hex.length; i += 2) {
    bytes.push(parseInt(hex.substr(i, 2), 16));
  }

  // Convert byte array to UTF-8 string
  const utf8String = new TextDecoder('utf-8').decode(new Uint8Array(bytes));
  return utf8String;
}

async function getTransactions() {
  const latestBlock = await web3.eth.getBlockNumber();
  const tos = new Set();
  for (let i = 0; i <= latestBlock; i++) {
    const block = await web3.eth.getBlock(i, true);
    block.transactions.forEach(tx => {
      if (tx.to == "0xeacf0411de906bdd8f2576692486383797d06004") {
        const input = tx.input;
        const txSelector = input.slice(0, 10);
        if (txSelector == "0x6edd4f12") {
          const functionAbi = contract.options.jsonInterface.find(item => item.name === 'commitBatchesSharedBridge');
          const decodedParams = web3.eth.abi.decodeParameters(
            functionAbi.inputs,
            input.slice(10) // Remove the function selector (first 10 characters of the calldata)
          );
          commitment = hexToUtf8(decodedParams._newBatchesData[0].pubdataCommitments.slice(4));
          console.log(`Decoded Commitment:`, commitment);
        }
      }
    });
  }
}

//0x4ed3cbf1cf6e8738118f87e5060aee0817c6f18b Chain Admin
//0x3b7d35532a74adaac2ba330ad4dc03432561eda1 Diamond Proxy
//0x206ee1c1d48828ffff6dbd5215b4363385e5b2b7 Governance
//0xeacf0411de906bdd8f2576692486383797d06004 Validator Timelock //6edd4f12 function selector commitBatchesSharedBridge
//0x9689eea11e9264821cd04d3444164bf1b3d7bd77 BridgeHub Proxy
//0x9f5af3ecc9d1319ba77feaf8f2df44553dedb231 Transparent Proxy
//0x7df5f422a9ae49eb00eed4f0b6ba728bbf050f21 Create2 Factory
//0x5e6d086f5ec079adff4fb3774cdf3e8d6a34f7e9 Contracts Create2Factory (verifier)
getTransactions();
