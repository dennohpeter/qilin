use dotenv::dotenv;
pub mod index;
pub mod uni_math;
pub mod utils;

fn main() {
    dotenv();
    index::init();
}
