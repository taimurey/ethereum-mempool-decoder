use ethabi::Contract;
use ethers::{
    types::{Address, Bytes, H160, H256, U256},
    utils::WEI_IN_ETHER,
};
use ethers_providers::{Middleware, Provider, StreamExt, Ws};
use log::{error, info};
use std::{env, process, str::FromStr, sync::Arc};
use tokio::time::Duration;
use url::Url;

use crate::{mempool::decoder::decode_transaction_input, types::settings::Settings};

use super::{
    decoder::input_decoder,
    utils::{UNISWAP_V2_ABI, UNISWAP_V3_ABI},
};

// async fn update_transaction_details(
//     transaction_details: Arc<Mutex<TransactionDetails>>,
//     transaction: Transaction,
//     decoded_addr: Address,
// ) {
//     let mut details = transaction_details.lock().await;

//     details.update(
//         transaction.input.clone().to_vec(),
//         transaction.value.as_u64(),
//         decoded_addr,
//     );
// }

pub async fn mempool_listener(config: Settings) -> Result<(), Box<dyn std::error::Error>> {
    let wss_node_endpoint = config.connection.wss_node_endpoint;

    let uniswap_v3_router_contract = Arc::new(
        Contract::load(UNISWAP_V3_ABI.as_bytes())
            .expect("Failed to load Uniswap Router contract ABI"),
    );

    let ws = Ws::connect(wss_node_endpoint).await?;
    let url = Url::parse(&config.connection.ethereum_rpc_url).expect("Invalid URL");
    let connection = ethers_providers::Http::new(url);

    // make it 0 on private Node
    let provider = Arc::new(Provider::new(ws).interval(Duration::from_millis(10)));

    let http_provider = Arc::new(Provider::new(connection).interval(Duration::from_millis(1000)));

    let uniswap_v3_router: Address = H160::from_str(&config.contract.uniswap_v3_router)
        .unwrap()
        .into();

    let value_threshold: U256 = WEI_IN_ETHER * (1 / 1000);

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

        tokio::task::spawn(async move {
            if let Ok(transaction_option) = http_provider.get_transaction(transaction_hash).await {
                if let Some(transaction) = transaction_option {
                    if let Some(transaction_to) = transaction.to {
                        // println!("Transaction: {:#?}", transaction.input);
                        let _ = match input_decoder(transaction.input).await {
                            Ok(_) => (),
                            Err(e) => error!("Error decoding transaction input: {:?}", e),
                        };
                        // info!("Hash: {:?}", transaction.hash);
                        // if (transaction_to == uniswap_v3_router)
                        //     && transaction.value >= value_threshold
                        // {
                        //     println!("--------------------------------------------------------------------------------");
                        //     info!("Hash: {:?}", transaction.hash);

                        //     let contract = if transaction.to == Some(uniswap_v3_router) {
                        //         &uniswap_v3_router_contract
                        //     } else {
                        //         return;
                        //     };

                        //     match decode_transaction_input(&transaction.input, contract).await {
                        //         Ok(Some((_, decoded_addr))) => {
                        //             info!("Decoding transaction input was successful.");
                        //             info!("Hash: {:?}", transaction.hash);
                        //             info!("Nonce: {:?}", transaction.nonce);
                        //             info!("From: {:?}", transaction.from);
                        //             info!("To: {:?}", transaction_to);
                        //             info!("Value: {:?}", transaction.value);
                        //             info!("Gas: {:?}", transaction.gas);
                        //             info!(
                        //                 "Transaction type: {:?}",
                        //                 Some(transaction.transaction_type)
                        //             );
                        //         }

                        //         Ok(None) => error!(
                        //             "Failed to decode transaction input: Function not found."
                        //         ),
                        //         Err(e) => {
                        //             error!("Failed to decode transaction input: {:?}", e)
                        //         }
                        //     }
                        // }
                    }
                }
            }
        });
    }

    Ok(())
}
