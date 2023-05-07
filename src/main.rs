use dotenv::dotenv;
pub mod uni_math;
pub mod index;
pub mod utils;

fn main() {
    dotenv(); 
    index::init();
}