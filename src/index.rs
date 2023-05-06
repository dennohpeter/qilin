use dotenv::dotenv;
use std::env;
use ethers::core::{
    rand::thread_rng, 
    types::TransactionRequest, types::transaction::eip2718::TypedTransaction};
use ethers::prelude::*;
use ethers::providers::{Provider, Ws};
use ethers::core::k256::SecretKey;
use ethers_flashbots::{BundleRequest, FlashbotsMiddleware};
use eyre::Result;
use std::convert::TryFrom;
use url::Url;
use ethers_signers::{LocalWallet, Signer};

#[tokio::main]
pub async fn first_fn() -> Result<()> {
    // data collection
    dotenv().ok(); 
    let _infura_key = env::var("INFURA_API_KEY").clone().unwrap();
    let _uniswap_v3_address = env::var("V3FACTORY_ADDRESS").clone().unwrap();
    let _dai_address = env::var("DAI_ADDRESS").clone().unwrap();
    let _usdc_address = env::var("USDC_ADDRESS").clone().unwrap();
    let _usdt_address = env::var("USDT_ADDRESS").clone().unwrap();
    let _weth_address = env::var("WETH_ADDRESS").clone().unwrap();
    let _null_address = env::var("NULL_ADDRESS").clone().unwrap();
    /*
    What we care about:
    - Pool transfers to/from router, from watching pool
    - Token transfers to/from router, from watching token
    - Burn when applicable, from watching token
    - Rebase when applicable, from watching token
    - Sync when applicable, from watching token
    - Technically we need to figure out how to deal with the math but w/e for now


    configure our subscription(s)
    */

    // for WETH address need to check current request and pool via weth9 function
    // from router contract
    let http_provider_sepolia = Provider::try_from(format!("https://sepolia.infura.io/v3/{}", _infura_key))?;
    let http_provider = Provider::try_from(format!("https://mainnet.infura.io/v3/{}", _infura_key))?;

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

    // change commented sections to config file options
    // let client_sepolia = SignerMiddleware::new(
    //     FlashbotsMiddleware::new(
    //         http_provider_sepolia,
    //         Url::parse("https://relay-sepolia.flashbots.net")?,
    //         bundle_signer.unwrap(),
    //     ),
    //     wallet,
    // );

    let client = SignerMiddleware::new(
        FlashbotsMiddleware::new(
            http_provider,
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

    
    println!(
        "{0: <42} | {1: <42} | {2: <20} | {3: <10} | {4: <66} | {5: <66}",
        "to", 
        "from", 
        "value", 
        "gas", 
        "hash", 
        "calldata"
    );
    //let mut stream = ws_provider.pending_bundle(msg).await?;
    while let Some(tx_hash) = stream.next().await {
        let mut msg = ws_provider.get_transaction(tx_hash).await?;
        let data = msg.clone().unwrap_or(Transaction::default());

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
        println!("{:?}", data);

        // let tx: TypedTransaction = TransactionRequest::pay("vitalik.eth", 1).into();
        // let signature = client.signer().sign_transaction(&tx).await?;
        // let mut bundle = BundleRequest::new();
        // bundle.add_transaction(
        //     tx.rlp_signed(&signature)
        // );
    
        //let bundle = bundle.set_block(block.number.unwrap()+1).set_simulation_block(block.number.unwrap()).set_simulation_timestamp(0);

        //sending bundle
        // let pending_bundle = client.inner().send_bundle(&bundle).await?;
    }
    
    

     Ok(())
}

/*
Some(Transaction { 
    hash: 0x736a6a6abea12465de1cc4d73e9d0633203d30170aadf826a77c3f6d154d7732, 
    nonce: 21000, 
    block_hash: None, block_number: None, transaction_index: None, 
    from: 0xaabb8c0deb1270151b9b0776bbf9c890cd877e67, 
    to: Some(0x53844f9577c2334e541aec7df7174ece5df1fcf0), 
    value: 0, 
    gas_price: Some(1000000000), 
    gas: 3000000, 
    input: Bytes(0x9ed93318000000000000000000000000aabb8c0deb1270151b9b0776bbf9c890cd877e67), 
    v: 22310258, 
    r: 52120736319054986821632375614197023773675339739065370707613673340985280157230, 
    s: 2345234057493645648343460763188633139081893918229592397379631637369603903814, transaction_type: Some(0), access_list: None, max_priority_fee_per_gas: None, max_fee_per_gas: None, chain_id: Some(11155111), other: OtherFields { inner: {} } })

    // monitor works
    // monitor specific address
    // parse the data to only the functions we want ... bytes4(keccak256(function_name(paramenters)))
*/
// Some(Transaction { 
//     hash: 0x69be97c0cda4d872ee5868bfcd7671a8acb915a4f7a7edba7b76f4d08dad7c08, 
//     nonce: 8848, 
//     block_hash: None, 
//     block_number: None, 
//     transaction_index: None, 
//     from: 0x007ab5199b6c57f7aa51bc3d0604a43505501a0c, 
//     to: Some(0x0328adcc26d7ee6a71843bdbf716b7b2b0b4ffa3), 
//     value: 1000000000000000, 
//     gas_price: Some(1500000014), 
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

1) determine what pairs we care about on uniswapv2/v3
    - WETH/USDT
    -WETH address: https://etherscan.io/token/0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2
    -USDT address: https://etherscan.io/address/0xdac17f958d2ee523a2206206994597c13d831ec7

2) fine addresses to monitor
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

3) monitor the mempool to look for tx that interact with v2 and v3 routers
    - get txHash from websocket
    - using txHash to req transaction data from Infura websocket
    - filter out everything but the tx that interact with v2 and v3 routers

4) once we found the tx that interact with v2 and v3 routers from step (3), 
    check the tx's calldata to make sure it matches swap() selector
    - find the bytes for swap() selectors on v2 and v3
        - v2: oxasbsaadsf, v3: 0xasdfasdf
    - match tx calldata found from step (3) to match swap() selectors on v2 and v3

5) determine the tokens-in and tokens-out part of the calldata
    - parse the calldata to find the portion that represents in-token and out-token


6) observe event(`Transfer`) then query pool reserves, then update the pool reserve
    - subscribe_event_by_type
        - event definition: `Transfer` event
    - query the reserve pool and update the local pool reserve variable


7) using token-in & token-out data, determine the effect on the pool reserves
    - simulate the effect if the swap goes through and get the ending pool state
        - for V2 and V3  
    

    Resource: UniV3 Math: https://crates.io/crates/uniswap_v3_math


8) bundle submission
    - TBD


*/
