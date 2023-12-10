mod loops;
mod channels;

use crate::loops::{accept_loop, Result};

use async_std::task;
use channels::{test_threads, spawn_thread, test_mutex};

#[tokio::main]
async fn main() -> Result<()> {
    std::thread::sleep(std::time::Duration::from_millis(1000));
    test_threads();

    println!("Starting new worker thread");
    spawn_thread();
    test_mutex();

    let fut = accept_loop("127.0.0.1:8080");
    task::block_on(fut)
}
