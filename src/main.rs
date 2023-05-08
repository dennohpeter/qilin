use dotenv::dotenv;
pub mod abigen;
pub mod bindings;
pub mod index;
pub mod uni_math;
pub mod utils;

fn main() {
    dotenv();
    index::init();
}
