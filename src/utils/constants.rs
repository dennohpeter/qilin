pub const DAI_ADDRESS: &str = "0x6b175474e89094c44da98b954eedeac495271d0f";
pub const USDC_ADDRESS: &str = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";
pub const USDT_ADDRESS: &str = "0xdAC17F958D2ee523a2206206994597C13D831ec7";
pub const WETH_ADDRESS: &str = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";
pub const NULL_ADDRESS: &str = "0x0000000000000000000000000000000000000000";
pub const V3ROUTER_ADDRESS: &str = "0xe592427a0aece92de3edee1f18e0157c05861564";
pub const UNISWAP_V3_ROUTER_1: &str = "0xE592427A0AEce92De3Edee1F18E0157C05861564";
pub const UNISWAP_V3_ROUTER_2: &str = "0x68b3465833fb72A70ecDF485E0e4C7bD8665Fc45";
pub const UNISWAP_V2_ROUTER_1: &str = "0xf164fC0Ec4E93095b804a4795bBe1e041497b92a";
pub const UNISWAP_V2_ROUTER_2: &str = "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D";
pub const UNISWAP_UNIVERSAL_ROUTER: &str = "0xEf1c6E67703c7BD7107eed8303Fbe6EC2554BF6B";

pub const SELECTOR_UNI: [&str; 3] = [
    "24856bc3", // "execute(bytes,bytes[])"
    "3593564c", // "execute(bytes,bytes[],uint256)"
    "fa461e33", // uniswapV3SwapCallback(int256,int256,bytes)
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
