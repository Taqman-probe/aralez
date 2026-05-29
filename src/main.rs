#[cfg(unix)]
use tikv_jemallocator::Jemalloc;

mod tls;
mod utils;
mod web;

#[cfg(unix)]
#[global_allocator]
static ALLOC: Jemalloc = Jemalloc;

#[cfg(windows)]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
// pub static A: CountingAllocator = CountingAllocator;

fn main() {
    web::start::run();
}
