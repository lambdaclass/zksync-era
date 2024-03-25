.PHONY: demo_validium_calldata demo_validium_blobs demo_rollup_calldata demo_rollup_blobs

demo_validium_calldata:
	zk && zk clean --all && zk env validium_calldata && zk init --validium-mode && zk server 

demo_validium_blobs:
	zk && zk clean --all && zk env validium_blobs && zk init --validium-mode && zk server 

demo_rollup_calldata:
	zk && zk clean --all && zk env rollup_calldata && zk init && zk server 

demo_rollup_blobs:
	zk && zk clean --all && zk env rollup_blobs && zk init  && zk server 
