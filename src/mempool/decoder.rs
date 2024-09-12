use std::{error::Error, fs};

use crate::types::logger::{log_decoded_input, token_to_string};

use super::utils::UNIVERSAL_FUNCTION_MAPPING;
use ethabi::Contract;
use ethers::types::Bytes;

lazy_static::lazy_static! {
    pub static ref TARGET_POOL_ABI: String = fs::read_to_string("./Pool/WBTC-ETH.json")
        .expect("Unable to read TARGET POOL ABI file");

    pub static ref UNIVERSAL_ROUTER_ABI: String = fs::read_to_string("./uniswap/UniswapUniversal.json")
        .expect("Unable to read Uniswap Router ABI file");

    pub static ref UNISWAP_V3_ROUTER_V2: String = fs::read_to_string("./uniswap/UniswapV3RouterRouter2.json")
        .expect("Unable to read Uniswap V3 Router ABI file");
}

pub async fn input_decoder(input: Bytes) -> Result<(), Box<dyn Error>> {
    if input.len() < 4 {
        return Ok(());
    }

    let pool_contract = Contract::load(TARGET_POOL_ABI.as_bytes())?;
    let universal_contract = Contract::load(UNIVERSAL_ROUTER_ABI.as_bytes())?;
    let uniswap_v3_router_2 = Contract::load(UNISWAP_V3_ROUTER_V2.as_bytes())?;

    let signature = &input[0..4];

    let function_name = match UNIVERSAL_FUNCTION_MAPPING.get(signature) {
        Some(name) => name,
        None => return Ok(()),
    };

    let function = match *function_name {
        "mixSwap" => pool_contract.function(function_name)?,
        "execute" => universal_contract.function(function_name)?,
        "exactInputSingle" => uniswap_v3_router_2.function(function_name)?,
        "multicall" => uniswap_v3_router_2.function(function_name)?,
        _ => return Ok(()),
    };

    let data = &input[4..];

    let tokens = function.decode_input(data)?;

    let types = function.inputs.clone();

    if types.len() != tokens.len() {
        return Err("Mismatch between input types and decoded tokens.".into());
    }

    let result = types
        .iter()
        .zip(tokens.iter())
        .map(|(ty, to)| format!("{} {}", ty.kind, token_to_string(to)))
        .collect::<Vec<String>>()
        .join("\n");

    log_decoded_input(function_name, &result);

    Ok(())
}
