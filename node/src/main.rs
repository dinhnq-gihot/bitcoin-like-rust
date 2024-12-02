mod handler;
mod util;

use {
    anyhow::Result,
    argh::FromArgs,
    btclib::types::blockchain::Blockchain,
    dashmap::DashMap,
    static_init::dynamic,
    std::path::Path,
    tokio::{
        net::{
            TcpListener,
            TcpStream,
        },
        sync::RwLock,
    },
};

#[dynamic]
pub static BLOCKCHAIN: RwLock<Blockchain> = RwLock::new(Blockchain::new());

// Node pool
#[dynamic]
pub static NODES: DashMap<String, TcpStream> = DashMap::new();

#[derive(FromArgs)]
/// Command line arguments for the node.
struct Args {
    /// The port number to listen on.
    #[argh(
        option,
        default = "9000",
        description = "the port number to listen on."
    )]
    port: u16,
    /// The path to the blockchain file.
    #[argh(
        option,
        default = "String::from(\"./blockchain.cbor\")",
        description = "the path to the blockchain file."
    )]
    blockchain_file: String,
    /// A list of node addresses to connect to.
    #[argh(positional, description = "A list of node addresses to connect to.")]
    nodes: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Args = argh::from_env();

    let port = args.port;
    let blockchain_file = args.blockchain_file;
    let nodes = args.nodes;

    if Path::new(&blockchain_file).exists() {
        util::load_blockchain(&blockchain_file).await?;
    } else {
        println!("blockchain file does not exist!");
        util::populate_connections(&nodes).await?;
        println!("total amount of known nodes: {}", NODES.len());
        if nodes.is_empty() {
            println!("no initial nodes provided, starting as a seed node");
        } else {
            let (longest_name, longest_count) = util::find_longest_chain_node().await?;
            // request the blockchain from the node with the longest blockchain
            util::download_blockchain(&longest_name, longest_count).await?;
            println!("blockchain downloaded from {longest_name}");
            // recalculate utxos
            {
                let mut blockchain = BLOCKCHAIN.write().await;
                blockchain.rebuild_utxos();
            }
            // try to adjust difficulty
            {
                let mut blockchain = BLOCKCHAIN.write().await;
                blockchain.try_adjust_target();
            }
        }
    }

    let addr = format!("0.0.0.0:{port}");
    let listener = TcpListener::bind(&addr).await?;
    println!("Listening on {addr}");

    tokio::spawn(util::cleanup());
    tokio::spawn(util::save(blockchain_file.clone()));

    loop {
        let (socket, _) = listener.accept().await?;
        tokio::spawn(handler::handle_connection(socket));
    }
}
