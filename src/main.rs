use dotenv::dotenv;
pub mod index;
pub mod uni_math;
pub mod utils;

use std::env;

fn main() {

    dotenv();


    let arg: Vec<String> = env::args().collect();

    let first_arg = arg[1].clone();

    if first_arg.contains("abigen") {
        println!("abigen");
    } else {
        println!("Command not Recognized");
    }

    index::init();

}
