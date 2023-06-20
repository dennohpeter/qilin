// SPDX-License-Identifier: MIT
pragma solidity ^0.8.15;

import {HuffDeployer} from "foundry-huff/HuffDeployer.sol";
import {IWETH} from "../interfaces/IWETH.sol";
import "forge-std/Script.sol";

contract SandwichDeployer is Script {

	address public sandwich;
	IWETH weth = IWETH(0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2);

	function run() public returns (address) {

		// deploy sandwich.huff contract
		sandwich = HuffDeployer.deploy("sandwich");


		uint wethFundAmount = msg.value;

		// fund sandwich
		weth.deposit{value: wethFundAmount}();
		weth.transfer(sandwich, wethFundAmount);

		return sandwich;

	}
}
