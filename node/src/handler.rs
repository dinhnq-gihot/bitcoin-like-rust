use {
    btclib::{
        network::Message,
        sha256::Hash,
        types::{
            block::Block,
            block_header::BlockHeader,
            transaction::{
                Transaction,
                TransactionOutput,
            },
        },
        util::MerkleRoot,
    },
    chrono::Utc,
    tokio::net::TcpStream,
};

pub async fn handle_connection(mut socket: TcpStream) {
    loop {
        // read a message from the socket
        let message = match Message::receive_async(&mut socket).await {
            Ok(message) => message,
            Err(e) => {
                println!("Invalid message from peer: {e}, closing the connection");
                return;
            }
        };

        use btclib::network::Message::*;
        match message {
            UTXOs(_) | Template(_) | Difference(_) | TemplateValidity(_) | NodeList(_) => {
                println!("");
                return;
            }
            FetchBlock(height) => {
                let blockchain = crate::BLOCKCHAIN.read().await;
                let Some(block) = blockchain.blocks().nth(height).cloned() else {
                    return;
                };
                let message = NewBlock(block);
                message.send_async(&mut socket).await.unwrap();
            }
            FetchUTXOs(key) => {
                println!("received request to fetch UTXOs");
                let blockchain = crate::BLOCKCHAIN.read().await;
                let utxos = blockchain
                    .utxos()
                    .iter()
                    .filter(|(_, (_, txout))| txout.pubkey == key)
                    .map(|(_, (marked, txout))| (txout.clone(), *marked))
                    .collect::<Vec<_>>();
                let message = UTXOs(utxos);
                message.send_async(&mut socket).await.unwrap();
            }
            SubmitTransaction(transaction) => {
                println!("submit tx");
                let mut blockchain = crate::BLOCKCHAIN.write().await;
                if let Err(e) = blockchain.add_to_mempool(transaction.clone()) {
                    println!("transaction rejected, closing connection: {e}");
                    return;
                }
                println!("added transaction to mempool");
                let nodes = crate::NODES
                    .iter()
                    .map(|x| x.key().clone())
                    .collect::<Vec<_>>();
                for node in nodes.iter() {
                    println!("sending to friend: {node}");
                    if let Some(mut stream) = crate::NODES.get_mut(node) {
                        let message = Message::NewTransaction(transaction.clone());
                        if message.send_async(&mut *stream).await.is_err() {
                            println!("failed to send transaction to {node}");
                        }
                    }
                }
                println!("transaction sent to friends");
            }
            NewTransaction(transaction) => {
                let mut blockchain = crate::BLOCKCHAIN.write().await;
                println!("received transaction from friend");
                if blockchain.add_to_mempool(transaction).is_err() {
                    println!("transaction rejected, closing connection");
                    return;
                }
            }
            FetchTemplate(pubkey) => {
                let blockchain = crate::BLOCKCHAIN.read().await;
                let mut txs = Vec::new();
                // insert txs from mempool
                txs.extend(
                    blockchain
                        .mempool()
                        .iter()
                        .take(btclib::BLOCK_TRANSACTION_CAP)
                        .map(|(_, tx)| tx)
                        .cloned()
                        .collect::<Vec<_>>(),
                );

                // insert coinbase tx with pubkey
                txs.insert(
                    0,
                    Transaction {
                        inputs: vec![],
                        outputs: vec![TransactionOutput {
                            pubkey,
                            value: 0,
                            unique_id: uuid::Uuid::new_v4(),
                        }],
                    },
                );

                let merkle_root = MerkleRoot::calculate(&txs);
                let mut block = Block::new(
                    BlockHeader {
                        timestamp: Utc::now(),
                        nonce: 0,
                        prev_block_hash: blockchain
                            .blocks()
                            .last()
                            .map(|last_block| last_block.hash())
                            .unwrap_or(Hash::zero()),
                        merkle_root,
                        target: blockchain.target(),
                    },
                    txs,
                );

                let miner_fees = match block.calculate_miner_fees(blockchain.utxos()) {
                    Ok(fees) => fees,
                    Err(e) => {
                        eprintln!("{e}");
                        return;
                    }
                };
                let reward = blockchain.calculate_block_reward();
                // update coinbase tx with reward
                block.transactions[0].outputs[0].value = reward + miner_fees;
                // recalculate merkle root
                block.header.merkle_root = MerkleRoot::calculate(&block.transactions);
                let message = Template(block);
                message.send_async(&mut socket).await.unwrap();
            }
            ValidateTemplate(block_template) => {
                let blockchain = crate::BLOCKCHAIN.read().await;
                let status = block_template.header.prev_block_hash
                    == blockchain
                        .blocks()
                        .last()
                        .map(|last_block| last_block.hash())
                        .unwrap_or(Hash::zero());
                let message = TemplateValidity(status);
                message.send_async(&mut socket).await.unwrap();
            }
            SubmitTemplate(block) => {
                println!("received allegedly mined template");
                let mut blockchain = crate::BLOCKCHAIN.write().await;
                if let Err(e) = blockchain.add_block(block.clone()) {
                    println!("block rejected: {e}, closing connection");
                    return;
                }
                blockchain.rebuild_utxos();
                println!("block look good, broadcasting");

                // send block to all friend nodes
                let nodes = crate::NODES
                    .iter()
                    .map(|x| x.key().clone())
                    .collect::<Vec<_>>();

                for node in nodes.iter() {
                    if let Some(mut stream) = crate::NODES.get_mut(node) {
                        let message = NewBlock(block.clone());
                        if message.send_async(&mut *stream).await.is_err() {
                            println!("failed to send block to {node}");
                        }
                    }
                }
            }
            DiscoverNodes => {
                let nodes = crate::NODES
                    .iter()
                    .map(|x| x.key().clone())
                    .collect::<Vec<_>>();
                let message = NodeList(nodes);
                message.send_async(&mut socket).await.unwrap();
            }
            AskDifference(height) => {
                let blockchain = crate::BLOCKCHAIN.read().await;
                let count = blockchain.block_height() as i32 - height as i32;
                let message = Difference(count);
                message.send_async(&mut socket).await.unwrap();
            }
            NewBlock(block) => {
                let mut blockchain = crate::BLOCKCHAIN.write().await;
                println!("received new block");
                if blockchain.add_block(block).is_err() {
                    println!("block rejected");
                }
            }
        }
    }
}
