use crate::gas_prices::{fetch_gas_prices, process_gas_prices};
use crate::{
    transaction_details::TransactionDetails, transactions::hex::decode, SMART_CONTRACT_ABI,
};
use ::log::info;

use ethers::abi::{Token, Tokenizable};
use ethers::{
    contract::Contract,
    core::types::transaction::eip2718::TypedTransaction,
    middleware::SignerMiddleware,
    prelude::*,
    providers::{Http, Middleware, Provider},
    signers::LocalWallet,
    types::{TransactionRequest, U256},
    utils::parse_ether,
};
use ethers_core::abi::InvalidOutputType;
pub use ethers_core::k256::SecretKey;
use ethers_core::rand::{distributions::Uniform, Rng, SeedableRng};
use ethers_flashbots::{BundleRequest, FlashbotsMiddleware, PendingBundleError};
pub use eyre::Result;
pub use hex;
use std::{
    convert::{From, TryFrom, TryInto},
    env,
    str::FromStr,
    sync::Arc,
};
use tokio::sync::Mutex;
use url::Url;

#[derive(Clone, Debug)]
pub struct Permit {
    pub v: u64,
    pub r: H256,
    pub s: H256,
}

impl Tokenizable for Permit {
    fn into_token(self) -> Token {
        Token::Tuple(vec![
            Token::Uint(self.v.into()),
            Token::FixedBytes(self.r.0.to_vec()),
            Token::FixedBytes(self.s.0.to_vec()),
        ])
    }

    fn from_token(token: Token) -> Result<Self, InvalidOutputType> {
        if let Token::Tuple(mut items) = token {
            let s = if let Token::FixedBytes(s) = items
                .pop()
                .ok_or(InvalidOutputType("Expected FixedBytes for s".into()))?
            {
                H256::from_slice(&s)
            } else {
                return Err(InvalidOutputType("Expected FixedBytes for s".into()));
            };
            let r = if let Token::FixedBytes(r) = items
                .pop()
                .ok_or(InvalidOutputType("Expected FixedBytes for r".into()))?
            {
                H256::from_slice(&r)
            } else {
                return Err(InvalidOutputType("Expected FixedBytes for r".into()));
            };
            let v = if let Token::Uint(v) = items
                .pop()
                .ok_or(InvalidOutputType("Expected Uint for v".into()))?
            {
                v.as_u64()
            } else {
                return Err(InvalidOutputType("Expected Uint for v".into()));
            };
            Ok(Permit { v, r, s })
        } else {
            Err(InvalidOutputType("Expected Tuple for Permit".into()))
        }
    }
}

//Configuration struct to hold the private keys
struct Configuration {
    bundle_signer_private_key: String,
    wallet_private_key: String,
}

// async fn fetch_gas_price() -> Result<Value, Error> {
//     let key = "347ea251ffad40dea2423091041c10bc"; // replace with your key
//     let url = format!("https://api.owlracle.info/v4/goerli/gas?apikey={}", key);

//     let response = reqwest::get(&url).await?;
//     let gas_price: Value = response.json().await?;

//     Ok(gas_price)
// }

fn get_configuration() -> eyre::Result<Configuration> {
    let bundle_signer_private_key = env::var("BUNDLE_SIGNER_PRIVATE_KEY")
        .map_err(|_| eyre::eyre!("Failed to read BUNDLE_SIGNER_PRIVATE_KEY"))?;
    let wallet_private_key = env::var("WALLET_PRIVATE_KEY")
        .map_err(|_| eyre::eyre!("Failed to read WALLET_PRIVATE_KEY"))?;

    Ok(Configuration {
        bundle_signer_private_key,
        wallet_private_key,
    })
}

async fn swap_exact_eth_for_tokens_v2(
    contract: &Contract<
        SignerMiddleware<FlashbotsMiddleware<Provider<Http>, LocalWallet>, LocalWallet>,
    >,
    min_tokens: U256,
    path: Vec<H160>,
    deadline: U256,
    value: U256,
) -> Result<(), eyre::Report> {
    let call = contract
        .method::<_, H256>(
            "swapExactETHForTokensV2",
            (min_tokens, path, contract.address(), deadline),
        )?
        .value(value);
    let pending_transaction = call.send().await?;
    let _transaction_receipt = pending_transaction.await?;
    Ok(())
}

pub async fn fetch_and_process_gas_prices() -> eyre::Result<f64> {
    let url = "https://api.blocknative.com/gasprices/blockprices";
    let auth_header = "348231ff-8218-45bd-a947-f3fe1b3951d1";
    let res = fetch_gas_prices(url, auth_header).await?;
    let gas_price =
        process_gas_prices(&res).ok_or_else(|| eyre::eyre!("Failed to process gas price"))?;
    Ok(gas_price)
}

pub async fn process_transactions(
    transaction_details: Arc<Mutex<TransactionDetails>>,
) -> eyre::Result<()> {
    let config = get_configuration()?;
    // Connect to the network
    let provider = Provider::<Http>::try_from("https://mainnet.eth.aragon.network")?;
    // let provider = Provider::<Http>::try_from(
    //       "https://solitary-crimson-dawn.ethereum-goerli.quiknode.pro/f6d37c7af824f15d62364b1a056956eebaa891e9/",
    //   )?;

    //mainnet gas price
    fetch_and_process_gas_prices().await?;
    // Parse the private keys from hexadecimal strings to bytes
    let bundle_signer_private_key_bytes = decode(&config.bundle_signer_private_key)
        .expect("Failed to decode BUNDLE_SIGNER_PRIVATE_KEY");
    let wallet_private_key_bytes =
        decode(&config.wallet_private_key).expect("Failed to decode WALLET_PRIVATE_KEY");

    // Then you can create the secret keys:
    let bundle_signer_secret_key = SecretKey::from_slice(&bundle_signer_private_key_bytes)
        .map_err(|_| eyre::eyre!("Failed to create SecretKey from BUNDLE_SIGNER_PRIVATE_KEY"))?;
    let wallet_secret_key = SecretKey::from_slice(&wallet_private_key_bytes)
        .map_err(|_| eyre::eyre!("Failed to create SecretKey from WALLET_PRIVATE_KEY"))?;

    let bundle_signer = LocalWallet::from(bundle_signer_secret_key);
    let wallet = LocalWallet::from(wallet_secret_key);

    //Contract
    let contract_address = H160::from_str("0x9BD9e0A2A3A585A2F39B56F34eE334a7360D45c0")?; // replace with your actual contract address

    let contract_abi = ethabi::Contract::load(SMART_CONTRACT_ABI.as_bytes())?;

    let contract = Contract::new(
        contract_address,
        contract_abi,
        SignerMiddleware::new(
            FlashbotsMiddleware::new(
                Provider::<Http>::try_from("https://mainnet.eth.aragon.network")?,
                Url::parse("https://relay.flashbots.net")?,
                //   Provider::<Http>::try_from("https://solitary-crimson-dawn.ethereum-goerli.quiknode.pro/f6d37c7af824f15d62364b1a056956eebaa891e9/")?,
                // Url::parse("https://relay-goerli.flashbots.net")?,
                bundle_signer.clone(), // clone here
            ),
            wallet.clone(),
        )
        .into(),
    );

    let eth_address = H160::from_str("0x3f39c5B139f3AdBf94eea553A7BC3E755a76adB7")?;
    let provider = NonceManagerMiddleware::new(provider, eth_address);
    let client = Arc::new(provider);

    // Add signer and Flashbots middleware
    let client = SignerMiddleware::new(
        FlashbotsMiddleware::new(
            client,
            Url::parse("https://relay.flashbots.net")?,
            // Url::parse("https://relay-goerli.flashbots.net")?,
            bundle_signer.clone(),
        ),
        wallet.clone(),
    );
    // get last block number
    let block_number = client.get_block_number().await?;

    // Assuming transaction_details is an Arc<Mutex<TransactionDetails>>
    // let transaction_details = Arc::new(Mutex::new(TransactionDetails::new()));

    // Random transaction value
    let mut rng = rand::rngs::StdRng::from_entropy();
    let min: u64 = 20000000000000000;
    let max: u64 = 100000000000000000;

    // Lock the mutex and read the transaction details
    let mut transaction_details = transaction_details.lock().await;

    // Generate a random value between `min` and `max` (20000000000000000 and 100000000000000000 in this case).
    let dist = Uniform::from(min..max);
    let random_value: u64 = rng.sample(dist);

    // If the random value is less than the shared mutex value, generate a new random value between `value` and `max`.
    transaction_details.value =
        if random_value < transaction_details.value && transaction_details.value < max {
            let dist_adjusted = Uniform::from(transaction_details.value..max);
            rng.sample(dist_adjusted)
        } else if random_value >= transaction_details.value && transaction_details.value < max {
            let dist_adjusted = Uniform::from(transaction_details.value..max);
            rng.sample(dist_adjusted)
        } else {
            random_value
        };

    let value = &transaction_details.value;
    info!("Transaction value: {:?}", value);

    let token_address = transaction_details.decoded_addr;
    info!("Token address: {:?}", token_address);

    let weth_address = H160::from_str("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2")?;

    let value_u256 = U256::from(*value);
    let slippage = value_u256 / U256::from(100);
    let min_tokens = value_u256 - slippage;

    let path = vec![weth_address, token_address];
    info!("Path: {:?}", path);
    let to = wallet.address();
    info!("Wallet address: {:?}", to);
    let deadline = U256::from((block_number + 100).as_u64());

    //Transaction Data
    let call = contract.method::<_, H256>(
        "swapExactETHForTokensV2",
        (min_tokens, path.clone(), deadline),
    )?;
    let transaction_data = call
        .calldata()
        .ok_or_else(|| eyre::eyre!("Failed to get calldata"))?;

    info!("Transaction data: {:?}", transaction_data);

    // let gas_price_info = fetch_gas_price().await?;
    // let gas_price = gas_price_info["fast"].as_u64().unwrap_or(590000);
    let gas_price_result = fetch_and_process_gas_prices().await;
    let gas_price = match gas_price_result {
        Ok(price) => ethers::core::types::U256::from((price * 1e9).round() as u64), // convert to GWei
        Err(e) => {
            return Err(eyre::Report::msg(format!(
                "Failed to fetch and process gas prices: {}",
                e
            )))
        }
    };

    // Build a custom bundle
    let tx = {
        let mut inner: TypedTransaction = TransactionRequest::new()
            .to(to)
            .value(*value)
            .gas_price(gas_price)
            .data(transaction_data)
            .chain_id(5)
            .into();
        client.fill_transaction(&mut inner, None).await?;
        inner
    };

    let signature = client.signer().sign_transaction(&tx).await?;

    let rlp_signed_tx = tx.rlp_signed(&signature);

    let bundle = BundleRequest::new()
        .push_transaction(rlp_signed_tx)
        .set_block(block_number + 1)
        .set_simulation_block(block_number)
        .set_simulation_timestamp(0);

    // Simulate it
    let simulated_bundle = client.inner().simulate_bundle(&bundle).await?;
    println!("Simulated bundle: {:?}", simulated_bundle);
    println!("About to send bundle...");

    // Send it
    let pending_bundle = client.inner().send_bundle(&bundle).await?;
    println!("Bundle sent...");

    // You can also optionally wait to see if the bundle was included
    match pending_bundle.await {
        Ok(bundle_hash) => println!(
            "Bundle with hash {:?} was included in target block",
            bundle_hash
        ),
        Err(PendingBundleError::BundleNotIncluded) => {
            println!("Bundle was not included in target block.")
        }
        Err(e) => println!("An error occured: {}", e),
    }

    swap_exact_eth_for_tokens_v2(&contract, min_tokens, path.clone(), deadline, value_u256).await?;

    // Wallet to which money will be transferred
    let threshold: U256 = parse_ether("1")?.try_into()?;

    // Query token balance
    let token_balance: U256 = contract
        .method::<_, U256>("getBalanceOf", (token_address, contract.address()))?
        .call()
        .await?;

    println!("Balance: {:?}", token_balance);

    let amounts_out = contract
        .method::<_, Vec<U256>>(
            "getAmountsOut",
            (token_balance, vec![token_address, weth_address]),
        )?
        .call()
        .await?;

    // Call swapTokensForWETHV2 function to swap BOT token to WETH
    let permit = Permit {
        v: 0,
        r: ethabi::ethereum_types::H256([0u8; 32]),
        s: ethabi::ethereum_types::H256([0u8; 32]),
    };

    let min_weth = amounts_out[1] * 95 / 100;

    let swap_method = contract.method::<_, H256>(
        "swapTokensForWETHV2",
        (
            token_address,
            token_balance,
            min_weth,
            vec![token_address, weth_address], // path
            deadline,
            permit.v,
            permit.r,
            permit.s,
        ),
    )?;
    let swap_tx = swap_method.send().await?;
    swap_tx.await?;

    // Then check the WETH balance
    let weth_balance: U256 = contract
        .method::<_, U256>("getBalanceOf", (weth_address, contract.address()))?
        .call()
        .await?;

    // If the balance is greater than the threshold, perform the swap to ETH
    if weth_balance > threshold {
        // Call swapWETHForETHV2 to swap WETH to ETH
        let swap_method = contract.method::<_, H256>(
            "swapWETHForETHV2",
            (
                weth_balance,
                amounts_out[1], // minimum amount of ETH you want to receive
                vec![weth_address, eth_address], // path
            ),
        )?;
        let swap_tx = swap_method.send().await?;
        swap_tx.await?;

        //Call contract to Withdraw ETH to owner
        let withdraw_method = contract.method::<_, H256>("withdrawEthToOwner", ())?;
        let withdraw_tx = withdraw_method.send().await?;
        withdraw_tx.await?;
    } else {
        println!("WETH balance is less than threshold");
    }
    Ok(())
}
