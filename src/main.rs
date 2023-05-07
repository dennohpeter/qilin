use dotenv::dotenv;
pub mod index;
pub mod uni_math;
pub mod utils;
pub mod abigen;

use std::env;

fn main() {

    dotenv();



    index::init();

}
