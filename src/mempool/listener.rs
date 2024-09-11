use colored::Colorize;
use ethers::{
    types::{Address, H160, U256},
    utils::WEI_IN_ETHER,
};
use ethers_providers::{Middleware, Provider, StreamExt, Ws};
use log::{error, info};
use std::time::Duration;
use std::{
    io::{self, Write},
    process,
    str::FromStr,
    sync::Arc,
};
use url::Url;

use crate::{mempool::decoder::input_decoder, types::settings::Settings};

pub async fn mempool_listener(config: Settings) -> Result<(), Box<dyn std::error::Error>> {
    let wss_node_endpoint = config.connection.wss_node_endpoint;

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

    info!("Listening to Pending Transactions{}", "...".red());

    let mut stream = match provider.watch_pending_transactions().await {
        Ok(stream) => stream,
        Err(e) => {
            error!("Failed to subscribe to pending transactions: {:?}", e);
            process::exit(1);
        }
    };
    // handle.join().unwrap();
    // stop_animation.store(true, Ordering::Relaxed);
    while let Some(transaction_hash) = stream.next().await {
        let http_provider = Arc::clone(&http_provider);

        // let transaction_hash =
        //     H256::from_str("0x4c35ca9cc4b7624c02f3ffa8862175582dee982ceb0c44a094e2dd346c5196b4")?;

        tokio::task::spawn(async move {
            if let Ok(transaction_option) = http_provider.get_transaction(transaction_hash).await {
                if let Some(transaction) = transaction_option {
                    if let Some(_transaction_to) = transaction.to {
                        match input_decoder(transaction.input).await {
                            Ok(s) => s,
                            Err(_) => {
                                return;
                            }
                        }

                        // if input.contains(&pool_address_str_clone) {
                        //     info!("Hash: {:?}", transaction.hash);
                        //     info!("Nonce: {:?}", transaction.nonce);
                        //     info!("From: {:?}", transaction.from);
                        // }
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
