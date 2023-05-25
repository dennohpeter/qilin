# Commands
1. to run
- `cargo run`

2. to test

- `cargo make test`

  - this command will spin up a local `Anvil` node, forking block 15686252 from Mainnet, for testing

3. to generate bindings in `/src/bindings` and ABIs in `/abi`

- `cargo make abigen`
  * need to specify the contract to be generated in `src/abigen.rs`

4. by default bot runs on Mainnet. To run on Goerli, 
- `cargo run goerli`
