use std::{collections::HashMap, fs};
use tiny_keccak::{Hasher, Keccak};

lazy_static::lazy_static! {
   pub static ref UNIVERSAL_FUNCTION_MAPPING: HashMap<[u8; 4], &'static str> = {
        let mut m = HashMap::new();
        m.insert(keccak256("collectRewards(bytes)"), "collectRewards");
        m.insert(keccak256("execute(bytes,bytes[])"), "execute");
        m.insert(keccak256("onERC1155BatchReceived(address,address,uint256[],uint256[],bytes)"), "onERC1155BatchReceived");
        m.insert(keccak256("onERC1155Received(address,address,uint256,uint256,bytes)"), "onERC1155Received");

        // Inserting existing function mappings
        m.insert([0x7f, 0xf3, 0x6a, 0xb5], "swapExactETHForTokens");
        m.insert([0xb6, 0xf9, 0xde, 0x95],"swapExactETHForTokensSupportingFeeOnTransferTokens");
        m.insert([0x38, 0xed, 0x17, 0x39], "swapExactTokensForTokens");
        m.insert([0x38, 0x75, 0x24, 0x3e], "swapTokensForExactTokens");
        m.insert([0x18, 0xcb, 0xaf, 0xe5], "swapExactTokensForETH");
        m.insert([0x4f, 0x36, 0x34, 0xf6], "swapETHForExactTokens");
        m.insert([0xf3, 0x02, 0xd7, 0x18], "swapTokensForExactETH");
        m.insert([0x1b, 0xb7, 0xab, 0x1d], "exactInput");
        m.insert([0x2e, 0x0a, 0x9e, 0x96], "exactInputSingle");
        m.insert([0x8b, 0x53, 0xb4, 0x36], "exactOutput");
        m.insert([0xd9, 0xc1, 0xad, 0xed], "exactOutputSingle");
        m.insert([48, 26, 55, 32], "mixSwap");
        m.insert([53, 147, 86, 76], "execute");

        m
    };

    pub static ref TARGET_POOL_MAPPING: HashMap<[u8; 4], &'static str> = {
        let mut m = HashMap::new();
        m.insert(keccak256("collectRewards(bytes)"), "collectRewards");
        m
    };

    pub static ref UNISWAP_V2_ABI: String = fs::read_to_string("./uniswap/UniswapV2Router.json")
        .expect("Unable to read Uniswap V2 Router ABI file");

    pub static ref UNISWAP_V3_ABI: String = fs::read_to_string("./uniswap/UniswapV3Router.json")
        .expect("Unable to read Uniswap V3 Router ABI file");
}

pub fn keccak256(input: &str) -> [u8; 4] {
    let mut hasher = Keccak::v256();
    hasher.update(input.as_bytes());
    let mut output = [0u8; 32];
    hasher.finalize(&mut output);
    [output[0], output[1], output[2], output[3]]
}
