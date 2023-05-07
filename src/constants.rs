pub const SELECTOR_UNI: [&str; 1] = [
    "24856bc3", // "execute(bytes,bytes[])"
    "3593564c", // "execute(bytes,bytes[],uint256)"
    "fa461e33"  // uniswapV3SwapCallback(int256,int256,bytes)
];

pub const SELECTOR_V3_R1: [&str; 1] = ["ac9650d8"]; // "multicall(bytes[])"

pub const SELECTOR_V3_R2: [&str; 5] = [
    "1f0464d1", // "multicall(bytes32,bytes[])"
    "5ae401dc", // "multicall(uint256,bytes[])"
    "ac9650d8", // "multicall(bytes[])"
    "472b43f3", // "swapExactTokensForTokens(uint256,uint256,address[],address)"
    "42712a67", // "swapTokensForExactTokens(uint256,uint256,address[],address)"
];

pub const SELECTOR_V2_R1: [&str; 6] = [
    "fb3bdb41", // "swapETHForExactTokens(uint256,address[],address,uint256)"
    "7ff36ab5", // "swapExactETHForTokens(uint256,address[],address,uint256)"
    "18cbafe5", // "swapExactTokensForETH(uint256,uint256,address[],address,uint256)"
    "38ed1739", // "swapExactTokensForTokens(uint256,uint256,address[],address,uint256)"
    "4a25d94a", // "swapTokensForExactETH(uint256,uint256,address[],address,uint256)"
    "8803dbee", // "swapTokensForExactTokens(uint256,uint256,address[],address,uint256)"
];

pub const SELECTOR_V2_R2: [&str; 9] = [
    "fb3bdb41", // "swapETHForExactTokens(uint256,address[],address,uint256)"
    "7ff36ab5", // "swapExactETHForTokens(uint256,address[],address,uint256)"
    "b6f9de95", // "swapExactETHForTokensSupportingFeeOnTransferTokens(uint256,address[],address,uint256)"
    "18cbafe5", // "swapExactTokensForETH(uint256,uint256,address[],address,uint256)"
    "791ac947", // "swapExactTokensForETHSupportingFeeOnTransferTokens(uint256,uint256,address[],address,uint256)"
    "38ed1739", // "swapExactTokensForTokens(uint256,uint256,address[],address,uint256)"
    "5c11d795", // "swapExactTokensForTokensSupportingFeeOnTransferTokens(uint256,uint256,address[],address,uint256)"
    "4a25d94a", // "swapTokensForExactETH(uint256,uint256,address[],address,uint256)"
    "8803dbee", // "swapTokensForExactTokens(uint256,uint256,address[],address,uint256)"
];
