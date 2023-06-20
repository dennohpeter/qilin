// SPDX-License-Identifier: MIT
pragma solidity >=0.8.15;

import "forge-std/Test.sol";
import "../../interfaces/IERC20.sol";
import "../../interfaces/IWETH.sol";
import "../Sandwicher.sol";

import "v2-core/interfaces/IUniswapV2Factory.sol";
import "v2-core/interfaces/IUniswapV2Router02.sol";
import "v2-core/interfaces/IUniswapV2Pair.sol";

contract SandwicherTest is DSTest {

	Sandwicher sandwicher;

	IUniswapV2Factory univ2Factory = IUniswapV2Factory(0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f);
	IUniswapV2Router02 univ2Router = IUniswapV2Router02(0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D);

	IWETH weth = IWETH(0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2);
	IERC20 usdc = IERC20(0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48);

	IUniswapV2Pair pair;

	function setUp() external {
		weth.deposit{value: 10e18}();
		pair = IUniswapV2Pair(
			univ2Factory.getPair(address(weth), address(usdc))
		);
	}

}