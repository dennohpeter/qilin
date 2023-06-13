
pub struct CallData {
    func_name: String,
    amount_out: U256,
    _to: String,
    deadline: U256,
    _path_size: U256,
    path: Vec<String>,
} 

impl CallData {
    pub fn new_swap_eth_for_exact_tokens(&self, input: Bytes) -> Self {

        let _amount_out = U256::from_big_endian(&input[4..36]);
        let _to = hex::encode(&input[36..68]);
        let _deadline = U256::from_big_endian(&input[68..100]);
        let _path_size = U256::from_little_endian(&input[100..132]);
        let _path = hex::encode(&input[132..(132+32*(_path_size).as_usize())]);

        CallData {
            func_name: "swapETHForExactTokens".to_string(),
            amount_out: _amount_out,
            _to: _to,
            deadline: _deadline,
            _path_size: _path_size,
            path: _path,
        }
    }

    pub fn new_swap_exact_eth_for_tokens(&self, input: Bytes) -> Self {
        // 7ff36ab5
        // swapExactETHForTokens(uint amountOutMin, address[] calldata path, address to, uint deadline)
        let amount_out_min = U256::from_big_endian(&input[4..36]);
        let _to = hex::encode(&input[36..68]);
        let _deadline = U256::from_big_endian(&input[68..100]);
        let _path_size = U256::from_big_endian(&input[100..132]);
        let _path = hex::encode(&input[132..(132+32*(_path_size).as_usize())]);

        CallData {
            func_name: "swapExactETHForTokens".to_string(),
            amount_out: amount_out_min,
            _to: _to,
            deadline: _deadline,
            _path_size: _path_size,
            path: _path,
        }

    }

    pub fn new_swap_exact_tokens_for_eth(&self, input: Bytes) -> Self {

        // 18cbafe5
        // swapExactTokensForETH(uint amountIn, uint amountOutMin, address[] calldata path, address to, uint deadline)
        let _amount_in = U256::from_big_endian(&input[4..36]);
        let _amount_out_min = U256::from_big_endian(&input[36..68]);
        let _to = hex::encode(&input[68..100]);
        let _deadline = U256::from_big_endian(&input[100..132]);
        let _path_size = U256::from_big_endian(&input[132..164]);
        let _path = hex::encode(&input[164..(164+32*(_path_size).as_usize())]);

        CallData {
            func_name: "swapExactTokensForETH".to_string(),
            amount_out: _amount_out_min,
            _to: _to,
            deadline: _deadline,
            _path_size: _path_size,
            path: _path,
        }
    }

    pub fn new_swap_exact_tokens_for_tokens(&self, input: Bytes) -> Self {

        // 38ed1739
        // function swapExactTokensForTokens(uint amountIn, uint amountOutMin, address[] calldata path, address to, uint deadline)
        let _amount_in = U256::from_big_endian(&input[4..36]);
        let _amount_out_min = U256::from_big_endian(&input[36..68]);
        let _to = hex::encode(&input[68..100]);
        let _deadline = U256::from_big_endian(&input[100..132]);
        let _path_size = U256::from_big_endian(&input[132..164]);
        let _path = hex::encode(&input[164..(164+32*(_path_size).as_usize())]);

        CallData {
            func_name: "swapExactTokensForTokens".to_string(),
            amount_out: _amount_out_min,
            _to: _to,
            deadline: _deadline,
            _path_size: _path_size,
            path: _path,
        }
    }

    pub fn new_swap_tokens_for_exact_eth(&self, input: Bytes) -> Self {

        // 4a25d94a
        // swapTokensForExactETH(uint amountOut, uint amountInMax, address[] calldata path, address to, uint deadline)
        let _amount_out = U256::from_big_endian(&input[4..36]);
        let _amount_in_max = U256::from_big_endian(&input[36..68]);
        let _to = hex::encode(&input[68..100]);
        let _deadline = U256::from_big_endian(&input[100..132]);
        let _path_size = U256::from_big_endian(&input[132..164]);
        let _path = hex::encode(&input[164..(164+32*(_path_size).as_usize())]);

        CallData {
            func_name: "swapTokensForExactETH".to_string(),
            amount_out: _amount_out,
            _to: _to,
            deadline: _deadline,
            _path_size: _path_size,
            path: _path,
        }
    }

    pub fn swap_tokens_for_exacact_tokens(&self, input: Bytes) -> Self {

        // 8803dbee
        // function swapTokensForExactTokens(uint amountOut, uint amountInMax, address[] calldata path, address to, uint deadline)
        let _amount_out = U256::from_big_endian(&input[4..36]);
        let _amount_in_max = U256::from_big_endian(&input[36..68]);
        let _to = hex::encode(&input[68..100]);
        let _deadline = U256::from_big_endian(&input[100..132]);
        let _path_size = U256::from_big_endian(&input[132..164]);
        let _path = hex::encode(&input[164..(164+32*(_path_size).as_usize())]);

        CallData {
            func_name: "swapTokensForExactTokens".to_string(),
            amount_out: _amount_out,
            _to: _to,
            deadline: _deadline,
            _path_size: _path_size,
            path: _path,
        }
    }

    pub fn swap_exact_eth_for_tokens_supporting_fee_on_transfer_tokens(&self, input: Bytes) -> Self {

        // b6f9de95
        // swapExactETHForTokensSupportingFeeOnTransferTokens(uint amountOutMin, address[] calldata path, address to, uint deadline)
        let _amount_out_min = U256::from_big_endian(&input[4..36]);
        let _to = hex::encode(&input[36..68]);
        let _deadline = U256::from_big_endian(&input[68..100]);
        let _path_size = U256::from_big_endian(&input[100..132]);
        let _path = hex::encode(&input[132..(132+32*(_path_size).as_usize())]);

        CallData {
            func_name: "swapExactETHForTokensSupportingFeeOnTransferTokens".to_string(),
            amount_out: _amount_out_min,
            _to: _to,
            deadline: _deadline,
            _path_size: _path_size,
            path: _path,
        }
    }

    pub fn swap_exact_tokens_for_tokens_supporting_fee_on_transfer_tokens(&self, input: Bytes) -> Self {

        // 791ac947
        // swapExactTokensForTokensSupportingFeeOnTransferTokens(uint amountIn, uint amountOutMin, address[] calldata path, address to, uint deadline)
        let _amount_in = U256::from_big_endian(&input[4..36]);
        let _amount_out_min = U256::from_big_endian(&input[36..68]);
        let _to = hex::encode(&input[68..100]);
        let _deadline = U256::from_big_endian(&input[100..132]);
        let _path_size = U256::from_big_endian(&input[132..164]);
        let _path = hex::encode(&input[164..(164+32*(_path_size).as_usize())]);

        CallData {
            func_name: "swapExactTokensForTokensSupportingFeeOnTransferTokens".to_string(),
            amount_out: _amount_out_min,
            _to: _to,
            deadline: _deadline,
            _path_size: _path_size,
            path: _path,
        }
    }

}
