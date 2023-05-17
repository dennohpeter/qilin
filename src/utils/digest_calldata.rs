// let _input = "0xfb3bdb41000000000000000000000000000000000000000000000000000000000007a1200000000000000000000000000000000000000000000000000000000000000080000000000000000000000000dac17f958d2ee523a2206206994597c13d831ec70000000000000000000000000000000000000000000000000000000005f5e0ff0000000000000000000000000000000000000000000000000000000000000006000000000000000000000000a0b86991c6218b36c1d19d4a2e9eb0ce3606eb48000000000000000000000000a0b86991c6218b36c1d19d4a2e9eb0ce3606eb48000000000000000000000000a0b86991c6218b36c1d19d4a2e9eb0ce3606eb48000000000000000000000000a0b86991c6218b36c1d19d4a2e9eb0ce3606eb48000000000000000000000000a0b86991c6218b36c1d19d4a2e9eb0ce3606eb48000000000000000000000000a0b86991c6218b36c1d19d4a2e9eb0ce3606eb48";
    // let input = Bytes::from(hex::decode(&_input[2..])?);
    // fb3bdb41
    // swapETHForExactTokens(uint amountOut, address[] calldata path, address to, uint deadline)
    // let _amount_out = U256::from_big_endian(&input[4..36]);
    // let _to = hex::encode(&input[36..68]);
    // let _deadline = U256::from_big_endian(&input[68..100]);
    // let _path_size = U256::from_little_endian(&input[100..132]);
    //let _path = hex::encode(&input[132..(132+32*(_path_size).as_usize())]);

    // // 7ff36ab5
    // // swapExactETHForTokens(uint amountOutMin, address[] calldata path, address to, uint deadline)
    // let amount_out_min = U256::from_big_endian(&input[4..36]);
    // let _to = hex::encode(&input[36..68]);
    // let _deadline = U256::from_big_endian(&input[68..100]);
    // let _path_size = U256::from_big_endian(&input[100..132]);
    // let _path = hex::encode(&input[132..(132+32*(_path_size).as_usize())]);

    // // 18cbafe5
    // // swapExactTokensForETH(uint amountIn, uint amountOutMin, address[] calldata path, address to, uint deadline)
    // let _amount_in = U256::from_big_endian(&input[4..36]);
    // let _amount_out_min = U256::from_big_endian(&input[36..68]);
    // let _to = hex::encode(&input[68..100]);
    // let _deadline = U256::from_big_endian(&input[100..132]);
    // let _path_size = U256::from_big_endian(&input[132..164]);
    // let _path = hex::encode(&input[164..(164+32*(_path_size).as_usize())]);

    // // 38ed1739
    // // function swapExactTokensForTokens(uint amountIn, uint amountOutMin, address[] calldata path, address to, uint deadline)
    // let _amount_in = U256::from_big_endian(&input[4..36]);
    // let _amount_out_min = U256::from_big_endian(&input[36..68]);
    // let _to = hex::encode(&input[68..100]);
    // let _deadline = U256::from_big_endian(&input[100..132]);
    // let _path_size = U256::from_big_endian(&input[132..164]);
    // let _path = hex::encode(&input[164..(164+32*(_path_size).as_usize())]);

    // // 4a25d94a
    // // swapTokensForExactETH(uint amountOut, uint amountInMax, address[] calldata path, address to, uint deadline)
    // let _amount_out = U256::from_big_endian(&input[4..36]);
    // let _amount_in_max = U256::from_big_endian(&input[36..68]);
    // let _to = hex::encode(&input[68..100]);
    // let _deadline = U256::from_big_endian(&input[100..132]);
    // let _path_size = U256::from_big_endian(&input[132..164]);
    // let _path = hex::encode(&input[164..(164+32*(_path_size).as_usize())]);

    // // 8803dbee
    // // function swapTokensForExactTokens(uint amountOut, uint amountInMax, address[] calldata path, address to, uint deadline)
    // let _amount_out = U256::from_big_endian(&input[4..36]);
    // let _amount_in_max = U256::from_big_endian(&input[36..68]);
    // let _to = hex::encode(&input[68..100]);
    // let _deadline = U256::from_big_endian(&input[100..132]);
    // let _path_size = U256::from_big_endian(&input[132..164]);
    // let _path = hex::encode(&input[164..(164+32*(_path_size).as_usize())]);

    // // V2 Only
    // // b6f9de95
    // // swapExactETHForTokensSupportingFeeOnTransferTokens(uint amountOutMin, address[] calldata path, address to, uint deadline)
    // let amount_out_min = U256::from_big_endian(&input[4..36]);
    // let _to = hex::encode(&input[36..68]);
    // let _deadline = U256::from_big_endian(&input[68..100]);
    // let _path_size = U256::from_big_endian(&input[100..132]);
    // let _path = hex::encode(&input[132..(132+32*(_path_size).as_usize())]);

    // // 791ac947
    // // swapExactTokensForETHSupportingFeeOnTransferTokens(uint amountIn, uint amountOutMin, address[] calldata path, address to, uint deadline)
    // let _amount_in = U256::from_big_endian(&input[4..36]);
    // let _amount_out_min = U256::from_big_endian(&input[36..68]);
    // let _to = hex::encode(&input[68..100]);
    // let _deadline = U256::from_big_endian(&input[100..132]);
    // let _path_size = U256::from_big_endian(&input[132..164]);
    // let _path = hex::encode(&input[164..(164+32*(_path_size).as_usize())]);

    // println!("amountOut: {:?}", _amount_out);
    // println!("to: {:?}", _to);
    // println!("deadline: {:?}", _deadline);
    // println!("Input: {:?}", hex::encode(&input[4..]));