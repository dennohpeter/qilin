use dotenv::dotenv;
pub mod abigen;
pub mod bindings;
pub mod uni_math;
pub mod utils;
use crate::utils::constants::{
    DAI_ADDRESS, NULL_ADDRESS, SELECTOR_UNI, SELECTOR_V2_R1, SELECTOR_V2_R2, SELECTOR_V3_R1,
    SELECTOR_V3_R2, UNISWAP_UNIVERSAL_ROUTER, UNISWAP_V2_ROUTER_1, UNISWAP_V2_ROUTER_2,
    UNISWAP_V3_ROUTER_1, UNISWAP_V3_ROUTER_2, USDC_ADDRESS, USDT_ADDRESS, WETH_ADDRESS,
};
use crate::utils::helpers::get_selectors;
use ethers::prelude::*;
use ethers::providers::{Provider, Ws};
use ethers::signers::{LocalWallet, Signer};
use ethers_flashbots::{BundleRequest, FlashbotsMiddleware};
use eyre::Result;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::env;
use std::error::Error;
use url::Url;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv()?;
    let arg: Vec<String> = env::args().collect();

    let first_arg = if arg.len() > 1 {
        arg[1].clone()
    } else {
        String::from("")
    };

    match first_arg.get(0..1) {
        Some(_) => {
            if first_arg.contains("abigen") {
                abigen::generate_abigen_for_addresses().await?;
                return Ok(());
            } else {
                println!("command not recognized");
            }
        }
        None => {
            println!("");
        }
    }

    // data collection
    let _infura_key = env::var("INFURA_API_KEY").clone().unwrap();
    let _etherscan_key = env::var("ETHERSCAN_API_KEY").clone().unwrap();
    let _dai_address = DAI_ADDRESS.parse::<H160>()?;
    let _usdc_address = USDC_ADDRESS.parse::<H160>()?;
    let _usdt_address = USDT_ADDRESS.parse::<H160>()?;
    let _weth_address = WETH_ADDRESS.parse::<H160>()?;
    let _null_address = NULL_ADDRESS.parse::<H160>()?;

    let uniswap_v3_router_1 = UNISWAP_V3_ROUTER_1.parse::<H160>()?;
    let uniswap_v3_router_2 = UNISWAP_V3_ROUTER_2.parse::<H160>()?;
    let uniswap_v2_router_1 = UNISWAP_V2_ROUTER_1.parse::<H160>()?;
    let uniswap_v2_router_2 = UNISWAP_V2_ROUTER_2.parse::<H160>()?;
    let uniswap_uni_router = UNISWAP_UNIVERSAL_ROUTER.parse::<H160>()?;

    let selectors_uni = get_selectors(&SELECTOR_UNI);
    let selectors_v3_r1 = get_selectors(&SELECTOR_V3_R1);
    let selectors_v3_r2 = get_selectors(&SELECTOR_V3_R2);
    let selectors_v2_r1 = get_selectors(&SELECTOR_V2_R1);
    let selectors_v2_r2 = get_selectors(&SELECTOR_V2_R2);

    // for WETH address need to check current request and pool via weth9 function
    // from router contract
    let http_provider_sepolia =
        Provider::try_from(format!("https://sepolia.infura.io/v3/{}", _infura_key))?;
    let http_provider =
        Provider::try_from(format!("https://mainnet.infura.io/v3/{}", _infura_key))?;

    // see: https://www.gakonst.com/ethers-rs/providers/ws.html
    let ws_url_sepolia = format!("wss://sepolia.infura.io/ws/v3/{}", _infura_key);
    let ws_url = format!("wss://mainnet.infura.io/ws/v3/{}", _infura_key);
    let ws_provider_sepolia = Provider::<Ws>::connect(ws_url_sepolia).await?;
    let ws_provider = Provider::<Ws>::connect(ws_url).await?;

    // bundle signing
    let _bundle_signer = env::var("FLASHBOTS_IDENTIFIER").clone().unwrap();
    let bundle_signer = _bundle_signer.parse::<LocalWallet>();
    //let addr = _bundle_signer.address();

    let _wallet = env::var("FLASHBOTS_SIGNER").clone().unwrap();
    let wallet = _wallet.parse::<LocalWallet>().unwrap();

    println!("Bundle Signer: {}", _bundle_signer);
    println!("Wallet: {}", _wallet);

    let client = SignerMiddleware::new(
        FlashbotsMiddleware::new(
            http_provider,
            //Url::parse("https://relay-sepolia.flashbots.net")?,
            Url::parse("https://relay.flashbots.net")?,
            bundle_signer.unwrap(),
        ),
        wallet,
    );

    // let seploia_endpoint = "https://api-sepolia.etherscan.io/";
    //     https://api-sepolia.etherscan.io/api
    //    ?module=transaction
    //    &action=getstatus
    //    &txhash=0x5d329954fae7d19b2fb9abf0e6862735243b1079c58e0ea307d7e933657ac083
    //    &apikey=YourApiKeyToken
    let mut stream = ws_provider.subscribe_pending_txs().await?;

    //let mut stream = ws_provider.pending_bundle(msg).await?;

    let routers = [
        (UNISWAP_UNIVERSAL_ROUTER, "Uniswap Univeral Router"),
        (UNISWAP_V3_ROUTER_1, "Uniswap V3 Router 1"),
        (UNISWAP_V3_ROUTER_2, "Uniswap V3 Router 2"),
        (UNISWAP_V2_ROUTER_1, "Uniswap V2 Router 1"),
        (UNISWAP_V2_ROUTER_2, "Uniswap V2 Router 2"),
    ];

    let mut router_selectors = HashMap::new();
    router_selectors.insert(UNISWAP_UNIVERSAL_ROUTER, &SELECTOR_UNI[..]);
    router_selectors.insert(UNISWAP_V3_ROUTER_1, &SELECTOR_V3_R1[..]);
    router_selectors.insert(UNISWAP_V3_ROUTER_2, &SELECTOR_V3_R2[..]);
    router_selectors.insert(UNISWAP_V2_ROUTER_1, &SELECTOR_V2_R1[..]);
    router_selectors.insert(UNISWAP_V2_ROUTER_2, &SELECTOR_V2_R2[..]);

    while let Some(tx_hash) = stream.next().await {
        let msg = ws_provider.get_transaction(tx_hash).await?;
        let data = msg.clone().unwrap_or(Transaction::default());
        let _to = data.to.clone().unwrap_or(_null_address);
        let _to = data.to.clone().unwrap_or(NULL_ADDRESS.parse::<H160>()?);

        let routers = [
            (&uniswap_uni_router, "Uniswap Univeral Router"),
            (&uniswap_v3_router_1, "Uniswap V3 Router 1"),
            (&uniswap_v3_router_2, "Uniswap V3 Router 2"),
            (&uniswap_v2_router_1, "Uniswap V2 Router 1"),
            (&uniswap_v2_router_2, "Uniswap V2 Router 2"),
        ];

        let mut router_selectors = HashMap::new();
        router_selectors.insert(uniswap_v3_router_1, &selectors_v3_r1);
        router_selectors.insert(uniswap_v3_router_2, &selectors_v3_r2);
        router_selectors.insert(uniswap_v2_router_1, &selectors_v2_r1);
        router_selectors.insert(uniswap_v2_router_2, &selectors_v2_r2);

        if data.input.len() >= 4 {
            if let Some((router_name, selectors)) =
                routers.iter().cloned().find_map(|(router, name)| {
                    router_selectors
                        .get(router)
                        .map(|selectors| (name, selectors))
                })
            {
                let first_four_bytes = &data.input[..4];
                for (i, selector) in selectors.iter().enumerate() {
                    let selector_slice = selector.as_ref();
                    if first_four_bytes.eq(selector_slice) {
                        println!("{}: Selector {} - {:?}", router_name, i, selector);
                    }
                }
            }
        }

        // if *_matched {
        //     println!("Selector ({:?}) for Router ({:?})", bytes_to_string(&data.input[..4]), router);
        //     _matched = &false;
        //ex (tracing rather then digesting)
        //"4a25d94a", // "swapTokensForExactETH(uint256,uint256,address[],address,uint256)"
        //bytes_to_string(&data.input[4..68])
        //bytes_to_string(&data.input[68..132])
        //bytes_to_string(&data.input[start..end]) // need to handle data size and memory size
        // data can reorder
        // }
        // we need to handle our logic for the selector calldata here
        // 1) split data
        // 2) convert the variables
        //      - numbers to ethers::types::{I256, U256};
        //      - addresses to H160
        // 3) ignore multicall but console log

        // let _to = data.to.clone().unwrap_or(_null_address.parse::<H160>()?);
        // let _from = data.from;
        // println!(
        //     "{:#x} | {:#x} | {2: <20} | {3: >10} | {4: <66} | {5: <66}",
        //     _to,
        //     _from,
        //     data.value,
        //     data.gas,
        //     data.hash,
        //     data.input
        // );
        //println!("{:?}", data);

        // let tx: TypedTransaction = TransactionRequest::pay("vitalik.eth", 1).into();
        // let signature = client.signer().sign_transaction(&tx).await?;
        // let mut bundle = BundleRequest::new();
        // bundle.add_transaction(
        //     tx.rlp_signed(&signature)
        // );

        //let bundle = bundle.set_block(block.number.unwrap()+1).set_simulation_block(block.number.unwrap()).set_simulation_timestamp(0);
    }
    Ok(())
}
//     gas: 21000,
//     input: Bytes(0x),
//     v: 0,
//     r: 108853379393575403114648895570501267920446440064797523802631923820109603901592,
//     s: 54432401851499039607746644666317390761051500462096442063488962792652443980717,
//     transaction_type: Some(2),
//     access_list: Some(AccessList([])),
//     max_priority_fee_per_gas: Some(1500000000),
//     max_fee_per_gas: Some(1500000014),
//     chain_id: Some(11155111),
//     other: OtherFields { inner: {} } })

// filter out 'to' targeted addresses we want

/*
- UniV2:WETH/USDT & UniV2:WETH/USDC & UniV2:USDT/USDC
- Pool A          & Pool B
T0 - 10 USDT           10 USDC
T1 - weth:10 USDT      11 USDC


Example:
- Assumption: Only V2
    - Token, A, B, and C
    - Pools: A/B, A/C, B/C
T0 - swap() -> A/C: A => C
        result - A($): up; C($): down

T1 - B => A, A => C, C => B

- Assumption: V2 and V3
    - Token A and B
    - Pools: A/B(v2), A/B(V3)

T0 - swap() -> A/B(V2): A => B
        result: A($): down; B($): up

T1 - Sell A buy B on V3 => sell B  buy A on V2

////////////////////////////////////////////////////////////////////////////////

1) ✅ determine what pairs we care about on uniswapv2/v3
    - WETH/USDT
    -WETH address: https://etherscan.io/token/0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2
    -USDT address: https://etherscan.io/address/0xdac17f958d2ee523a2206206994597c13d831ec7

2) ✅ fine addresses to monitor
    - find v2 router and v3 router addresses
    - find WETH and USDT addresses
    - WETH/USDT v2 pool address, and WETH/USDT v3 pool address
    ROUTERS = {
         "0xd9e1cE17f2641f24aE83637ab66a2cca9C378B9F": {
             "name": "Sushiswap: Router",
             "uniswap_version": 2,
             "factory_address": {2: "0xC0AEe478e3658e2610c5F7A4A2E1777cE9e4f2Ac"},
         },
        "0xf164fC0Ec4E93095b804a4795bBe1e041497b92a": {
            "name": "UniswapV2: Router",
            "uniswap_version": 2,
            "factory_address": {2: "0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f"},
        },
        "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D": {
            "name": "UniswapV2: Router 2",
            "uniswap_version": 2,
            "factory_address": {2: "0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f"},
        },
        "0xE592427A0AEce92De3Edee1F18E0157C05861564": {
            "name": "UniswapV3: Router",
            "uniswap_version": 3,
            "factory_address": {3: "0x1F98431c8aD98523631AE4a59f267346ea31F984"},
        },
        "0x68b3465833fb72A70ecDF485E0e4C7bD8665Fc45": {
            "name": "UniswapV3: Router 2",
            "uniswap_version": 3,
            "factory_address": {
                2: "0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f",
                3: "0x1F98431c8aD98523631AE4a59f267346ea31F984",
            },
        },
    }

    FACTORY_ADDRESSES = {
        'uniswap_v2': '0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f',
        'uniswap_v3': '0x1F98431c8aD98523631AE4a59f267346ea31F984',
         'uniswap_v3_router2': {
             '0x1F98431c8aD98523631AE4a59f267346ea31F984',
             '0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f'

         }
    }

3) ✅ monitor the mempool to look for tx that interact with v2 and v3 routers
    - ✅ get txHash from websocket
    - ✅ using txHash to req transaction data from Infura websocket
    - ✅ filter out everything but the tx that interact with v2 and v3 routers

4) ✅ once we found the tx that interact with v2 and v3 routers from step (3),
    check the tx's calldata to make sure it matches swap() selector
    - ✅ find the bytes for swap() selectors on v2 and v3
        - v2: oxasbsaadsf, v3: 0xasdfasdf
    - ✅ match tx calldata found from step (3) to match swap() selectors on v2 and v3

5) determine the tokens-in and tokens-out part of the calldata
    - parse the calldata to find the portion that represents in-token and out-token


6) observe event(`Transfer`) then query pool reserves, then update the pool reserve
    - subscribe_event_by_type
        - event definition: `event Transfer(address from, address to, unit256 value)` event
    - query the reserve pool and update the local pool reserve variable


7) using token-in & token-out data, determine the effect on the pool reserves
    - simulate the effect if the swap goes through and get the ending pool state
        - for V2
        -✅ V3


    Resource: UniV3 Math: https://crates.io/crates/uniswap_v3_math


8) bundle submission
    - TBD


V3 ROUTER1 SELECTORS TO WATCH
{
    "ac9650d8": "multicall(bytes[])"
}

V3 ROUTER2 SELECTORS TO WATCH
{
    "1f0464d1": "multicall(bytes32,bytes[])",
    "5ae401dc": "multicall(uint256,bytes[])",
    "ac9650d8": "multicall(bytes[])",
    "472b43f3": "swapExactTokensForTokens(uint256,uint256,address[],address)",
    "42712a67": "swapTokensForExactTokens(uint256,uint256,address[],address)"
}

V2 ROUTER01 SELECTORS TO WATCH
{
    "fb3bdb41": "swapETHForExactTokens(uint256,address[],address,uint256)",
    "7ff36ab5": "swapExactETHForTokens(uint256,address[],address,uint256)",
    "18cbafe5": "swapExactTokensForETH(uint256,uint256,address[],address,uint256)",
    "38ed1739": "swapExactTokensForTokens(uint256,uint256,address[],address,uint256)",
    "4a25d94a": "swapTokensForExactETH(uint256,uint256,address[],address,uint256)",
    "8803dbee": "swapTokensForExactTokens(uint256,uint256,address[],address,uint256)"
}

V2 ROUTER02 SELECTORS TO WATCH
{
    "fb3bdb41": "swapETHForExactTokens(uint256,address[],address,uint256)",
    "7ff36ab5": "swapExactETHForTokens(uint256,address[],address,uint256)",
    "b6f9de95": "swapExactETHForTokensSupportingFeeOnTransferTokens(uint256,address[],address,uint256)",
    "18cbafe5": "swapExactTokensForETH(uint256,uint256,address[],address,uint256)",
    "791ac947": "swapExactTokensForETHSupportingFeeOnTransferTokens(uint256,uint256,address[],address,uint256)",
    "38ed1739": "swapExactTokensForTokens(uint256,uint256,address[],address,uint256)",
    "5c11d795": "swapExactTokensForTokensSupportingFeeOnTransferTokens(uint256,uint256,address[],address,uint256)",
    "4a25d94a": "swapTokensForExactETH(uint256,uint256,address[],address,uint256)",
    "8803dbee": "swapTokensForExactTokens(uint256,uint256,address[],address,uint256)"
}


Transaction {
    hash: 0x872be985300821f1c7c8d099b346276948bc84354a642ff3ecd774d602246b60,
    nonce: 41,
    block_hash: None,
    block_number: None,
    transaction_index: None,
    from: 0xba3b26154931be77bd44928e62eee66f34db2661,
    to: Some(0x68b3465833fb72a70ecdf485e0e4c7bd8665fc45),
    value: 1000000000000000000,
    gas_price: Some(125000000000),
    gas: 304258,
    input: Bytes(0x5ae401dc000000000000000000000000000000000000000000000000000000006455e6bb00000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000e4472b43f30000000000000000000000000000000000000000000000000de0b6b3a764000000000000000000000000000000000000000000012212eb87e90adba30bbcec270000000000000000000000000000000000000000000000000000000000000080000000000000000000000000ba3b26154931be77bd44928e62eee66f34db26610000000000000000000000000000000000000000000000000000000000000002000000000000000000000000c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2000000000000000000000000378e1be15be6d6d1f23cfe7090b6a77660dbf14d00000000000000000000000000000000000000000000000000000000),
    v: 1,
    r: 101320250276167864627564976104790592610069124837894492495663500246216674461521,
    s: 36519862551638435947037849073949578028978097248470881776598175722020804060769,
    transaction_type: Some(2),
    access_list: Some(AccessList([])),
    max_priority_fee_per_gas: Some(12500000000),
    max_fee_per_gas: Some(125000000000),
    chain_id: Some(1),
    other: OtherFields { inner: {} } }
*/
