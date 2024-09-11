use std::{error::Error, fs};

use super::utils::UNIVERSAL_FUNCTION_MAPPING;
use ethabi::{Contract, Error as EthabiError, Token};
use ethers::types::{Address, Bytes};
use log::{error, info};

lazy_static::lazy_static! {
    pub static ref TARGET_POOL_ABI: String = fs::read_to_string("./Pool/WBTC-ETH.json")
        .expect("Unable to read TARGET POOL ABI file");

    pub static ref UNIVERSAL_ROUTER_ABI: String = fs::read_to_string("./uniswap/UniswapUniversal.json")
        .expect("Unable to read Uniswap Router ABI file");

    pub static ref UNISWAP_V3_ROUTER_V2: String = fs::read_to_string("./uniswap/UniswapV3RouterRouter2.json")
        .expect("Unable to read Uniswap V3 Router ABI file");
}

pub async fn input_decoder(input: Bytes) -> Result<(), Box<dyn Error>> {
    // Ensure the input is long enough to contain the function signature
    if input.len() < 4 {
        return Ok(());
    }

    // Load the contract ABI
    let pool_contract = Contract::load(TARGET_POOL_ABI.as_bytes())?;
    let universal_contract = Contract::load(UNIVERSAL_ROUTER_ABI.as_bytes())?;
    let uniswap_v3_router_2 = Contract::load(UNISWAP_V3_ROUTER_V2.as_bytes())?;

    // Extract the first 4 bytes of the input as the function signature
    let signature = &input[0..4];

    println!("Signature: {:?}", signature);

    // Identify the correct function using the signature
    // If you have a mapping, use it here to find the function name
    let function_name = match UNIVERSAL_FUNCTION_MAPPING.get(signature) {
        Some(name) => name,
        None => return Ok(()),
    };

    let function = match *function_name {
        "mixSwap" => pool_contract.function(function_name)?,
        "execute" => universal_contract.function(function_name)?,
        "exactInputSingle" => uniswap_v3_router_2.function(function_name)?,
        _ => return Ok(()),
    };

    // Extract the input data, skipping the first 4 bytes (signature)
    let data = &input[4..];

    // Decode the input data according to the function's input types
    let tokens = function.decode_input(data)?;

    let types = function.inputs.clone();

    // Check if the number of tokens matches the number of expected types
    if types.len() != tokens.len() {
        return Err("Mismatch between input types and decoded tokens.".into());
    }

    let result = types
        .iter()
        .zip(tokens.iter())
        .map(|(ty, to)| format!("{} {}", ty.kind, to))
        .collect::<Vec<String>>()
        .join("\n");

    info!("Decoded input: {:#?}", result);

    Ok(())
}

pub async fn decode_transaction_input(
    input: &Bytes,
    contract: &Contract,
) -> Result<Option<(&'static str, Address)>, EthabiError> {
    if input.0.len() < 4 {
        return Err(EthabiError::InvalidData);
    }

    let signature = &input.0[0..4];

    let function_name = UNIVERSAL_FUNCTION_MAPPING.get(signature);

    match function_name {
        Some(name) => {
            let function = contract.function(name)?;
            let tokens = function.decode_input(&input.0[4..])?;

            match *name {
                "swapExactETHForTokens"
                | "swapExactETHForTokensSupportingFeeOnTransferTokens"
                | "swapETHForExactTokens" => {
                    info!("Method Name: {:?}", name);
                    let mut address = None;
                    if let Token::Array(path) = &tokens[1] {
                        let mut path_iter = path.iter().skip(1);
                        while let Some(token) = path_iter.next() {
                            if let Token::Address(addr) = token {
                                address = Some(*addr);
                                println!(
                                    "__________________________________________________________"
                                );
                                info!("Path: {:?}", addr);
                                // let url = format!("https://www.dexview.com/eth/{:?}", addr);
                                // if webbrowser::open(&url).is_ok() {
                                //     info!("Opened transaction in web browser: {}", url);
                                // } else {
                                //     error!("Failed to open transaction in web browser: {}", url);
                                // }

                                let message = format!(
                                    "0x{}",
                                    addr.to_fixed_bytes()
                                        .iter()
                                        .map(|byte| format!("{:02x}", byte))
                                        .collect::<String>()
                                );

                                println!("Message: {:#?}", message);
                            } else {
                                error!("Invalid token in path: {:?}", token);
                            }
                        }
                    } else {
                        error!("Invalid path in input: {:?}", tokens[1]);
                    }
                    let web3_address = if let Some(ethers_address) = address {
                        ethers_address
                    } else {
                        return Err(EthabiError::InvalidData);
                    };
                    Ok(Some((*name, web3_address)))
                }
                _ => Ok(None),
            }
        }
        None => Ok(None),
    }
}
