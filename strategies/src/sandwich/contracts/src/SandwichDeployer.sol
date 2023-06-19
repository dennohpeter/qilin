// SPDX-License-Identifier: MIT
pragma solidity ^0.8.15;

import {HuffDeployer} from "foundry-huff/HuffDeployer.sol";

contract SandwichDeployer {

	uint256 public wethFundAmount;
	address public sandwich;
	IWETH weth = IWETH(0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2);
	uint256 wethFundAmount = 1000000000 ether;

	constructor(uint256 _wethFundAmount) {
		wethFundAmount = _wethFundAmount;
		setUp();
	}

	function setUp() internal {

		// deploy sandwich.huff contract
		sandwich = HuffDeployer.deploy("sandwich");

		// fund sandwich
		weth.deposit{value: wethFundAmount}();
		weth.transfer(sandwich, wethFundAmount);

	}
}
