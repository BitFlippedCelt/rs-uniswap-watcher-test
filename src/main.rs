use anyhow::{Ok, Result};
use dotenv::dotenv;
use ethers::{
    abi::{decode, Abi},
    addressbook::Chain,
    contract::{Contract, ContractFactory},
    etherscan::Client,
    providers::{Http, Middleware, Provider, ProviderExt, StreamExt, Ws},
    types::Address,
    utils::hex::{self, FromHex},
};
use log::debug;

#[derive(Clone, Debug)]
struct Factory {
    address: Address,
    abi: Abi,
    name: String,
    version: u8,
}

#[derive(Clone, Debug)]
struct Router {
    address: Address,
    abi: Abi,
    name: String,
    version: u8,
    factory: Vec<Factory>,
}

/// Fetches the ABI for a contract address from Etherscan
/// and caches it in the .cache directory
/// Returns the ABI as a string if successful
async fn get_abi(address: Address) -> Result<Abi> {
    // Create the cache directory if it doesn't exist
    let cache_path = std::path::Path::new(".cache");
    if !cache_path.exists() {
        debug!("Creating cache directory");
        std::fs::create_dir(cache_path)?;
    }

    // Check if the ABI is cached and return it if it is
    let cache_file = cache_path.join(format!("{:?}.json", address));
    if cache_file.exists() {
        debug!("Using cached ABI for {}", address);
        let abi = std::fs::read_to_string(cache_file)?;
        return Ok(serde_json::from_str(&abi)?);
    }

    // Fetch the ABI from Etherscan
    let etherscan = Client::new(
        Chain::Mainnet,
        dotenv::var("ETHERSCAN_API_KEY").expect("ETHERSCAN_API_KEY missing"),
    )
    .expect("Could not create etherscan client");

    let abi = etherscan.contract_abi(address).await?;

    // Cache the ABI
    debug!("Caching ABI for {}", address.to_string());
    std::fs::write(cache_file, serde_json::to_string(&abi)?)?;

    Ok(abi)
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load the .env file
    dotenv().ok();

    // Configure the logger
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Initialize Provider
    let provider_ws =
        Provider::<Ws>::connect(&dotenv::var("ETH_WS_URL").expect("ETH_WS_URL missing")).await?;

    // Initialize Factory
    let factory_v2 = Factory {
        address: "0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f".parse()?,
        abi: get_abi("0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f".parse()?).await?,
        name: "Uniswap V2".to_string(),
        version: 2,
    };

    // Initialize Routers
    let routers = vec![
        Router {
            address: "0xf164fC0Ec4E93095b804a4795bBe1e041497b92a".parse()?,
            abi: get_abi("0xf164fC0Ec4E93095b804a4795bBe1e041497b92a".parse()?).await?,
            name: "Uniswap V2".to_string(),
            version: 2,
            factory: vec![factory_v2.clone()],
        },
        Router {
            address: "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D".parse()?,
            abi: get_abi("0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D".parse()?).await?,
            name: "Uniswap V2: Router 2".to_string(),
            version: 2,
            factory: vec![factory_v2],
        },
    ];

    let mut tx_stream = provider_ws.subscribe_pending_txs().await?;
    while let Some(hash) = tx_stream.next().await {
        if let Some(tx) = provider_ws.get_transaction(hash).await? {
            match Some(tx.to) {
                Some(to) => {
                    let to = to.unwrap();
                    if let Some(router) = routers.iter().find(|router| router.address == to) {
                        println!("Transaction to: {}", router.name);
                        let abi = router.abi.clone();

                        // Check if the transaction is a swap from ETH to a token
                        let input_data = &tx.input.0;
                        if input_data.starts_with(&<[u8; 12]>::from_hex("0x18cbafe5").unwrap()) {
                            // let function = abi.function("swapExactETHForTokens")?;
                            // let input = function.decode_input(input_data[10..].as_ref())?;

                            // let inputs = decode(
                            //     function.inputs //function.inputs.clone()[1..].as_ref(),
                            //     &hex::decode(&input_data[10..]).unwrap(),
                            // )?;
                            // function.decode_input(tx.input.0.as_ref())?;

                            log::info!("Transaction: {:?}", tx);
                        }
                    }
                }
                None => println!("Transaction to: None"),
            }
        }
    }
    Ok(())
}
