use colored::Colorize;
use ethabi::Contract;
use ethers::{
    types::{Address, H160, U256},
    utils::WEI_IN_ETHER,
};
use ethers_providers::{Middleware, Provider, StreamExt, Ws};
use log::{error, info};
use std::{
    io::{self, Write},
    process,
    str::FromStr,
    sync::Arc,
};
use std::{
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::Duration,
};
use url::Url;

use crate::types::settings::Settings;

use super::utils::UNISWAP_V3_ABI;

pub async fn mempool_listener(config: Settings) -> Result<(), Box<dyn std::error::Error>> {
    let wss_node_endpoint = config.connection.wss_node_endpoint;

    let uniswap_v3_router_contract = Arc::new(
        Contract::load(UNISWAP_V3_ABI.as_bytes())
            .expect("Failed to load Uniswap Router contract ABI"),
    );

    // Clone the address string to avoid lifetime issues
    let pool_address_str = config.contract.address.clone();
    let pool_address_str = pool_address_str
        .strip_prefix("0x")
        .unwrap_or(&pool_address_str);

    let ws = Ws::connect(wss_node_endpoint).await?;
    let url = Url::parse(&config.connection.ethereum_rpc_url).expect("Invalid URL");
    let connection = ethers_providers::Http::new(url);

    let provider = Arc::new(Provider::new(ws).interval(Duration::from_millis(10)));
    let http_provider = Arc::new(Provider::new(connection).interval(Duration::from_millis(100)));

    // Prefix unused variables with an underscore
    let _uniswap_v3_router: Address = H160::from_str(&config.contract.uniswap_v3_router)
        .unwrap()
        .into();

    let _value_threshold: U256 = WEI_IN_ETHER * (1 / 1000);

    // Shared atomic boolean flag to stop the animation
    let stop_animation = Arc::new(AtomicBool::new(false));
    let stop_animation_clone = Arc::clone(&stop_animation);

    // Start the dot animation in a separate thread
    let handle = thread::spawn(move || {
        let mut dots = String::new();
        let mut count = 0;
        while !stop_animation_clone.load(Ordering::Relaxed) {
            if count == 3 {
                dots.clear();
                count = 0;
            }
            dots.push('.');
            count += 1;
            clear_previous_line().unwrap();
            info!("Listening to Pending Transactions{}", dots.clone().red());
            thread::sleep(Duration::from_millis(300));
        }
    });

    let mut stream = match provider.watch_pending_transactions().await {
        Ok(stream) => stream,
        Err(e) => {
            error!("Failed to subscribe to pending transactions: {:?}", e);
            process::exit(1);
        }
    };

    while let Some(transaction_hash) = stream.next().await {
        let http_provider = Arc::clone(&http_provider);
        let uniswap_v3_router_contract = Arc::clone(&uniswap_v3_router_contract);
        let pool_address_str_clone = pool_address_str.to_string();

        tokio::task::spawn(async move {
            if let Ok(transaction_option) = http_provider.get_transaction(transaction_hash).await {
                if let Some(transaction) = transaction_option {
                    if let Some(_transaction_to) = transaction.to {
                        let input = hex::encode(transaction.input);
                        // println!("Input: {:?}", input);
                        //   println!("pool_address_str_clone: {:?}", pool_address_str_clone);

                        if input.contains(&pool_address_str_clone) {
                            info!("Hash: {:?}", transaction.hash);
                            info!("Nonce: {:?}", transaction.nonce);
                            info!("From: {:?}", transaction.from);
                        }
                    }
                }
            }
        });
    }

    Ok(())
}

pub fn clear_previous_line() -> io::Result<()> {
    let clear_line = "\x1b[1A\x1b[2K";
    io::stdout().write_all(clear_line.as_bytes())?;
    io::stdout().flush()
}
