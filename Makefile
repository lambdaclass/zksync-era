.PHONY: demo_validium_calldata demo_validium_blobs demo_rollup_calldata demo_rollup_blobs

demo_validium_calldata:
	zk
	zk clean --all
	zk config compile main_demo_validium_calldata
	zk env main_demo_validium_calldata
	zk init --validium-mode --run-observability
	zk server

demo_validium_blobs:
	zk && zk clean --all && zk env main_demo_validium_blobs && zk init --validium-mode --run-observability && zk server 

demo_rollup_calldata:
	zk
	zk clean --all
	zk config compile main_demo_rollup_calldata
	zk env main_demo_rollup_calldata
	zk init --run-observability
	zk server

demo_rollup_blobs:
	zk && zk clean --all && zk env main_demo_rollup_blobs && zk init --run-observability && zk server 
