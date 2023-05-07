use dotenv::dotenv;
pub mod uni_math;
pub mod index;

fn main() {
    dotenv(); 
    index::init();
}