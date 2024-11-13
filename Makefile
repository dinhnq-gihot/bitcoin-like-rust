run-tx-gen:
	cd btclib && cargo run --bin tx_gen ../tx.cbor

run-tx-print:
	cd btclib && cargo run --bin tx_print ../tx.cbor
	
run-block-gen:
	cd btclib && cargo run --bin block_gen ../block.cbor

run-block-print:
	cd btclib && cargo run --bin block_print ../block.cbor

run-miner:
	cd miner && cargo run ../block.cbor 100