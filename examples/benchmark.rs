//  Description:
//!   Runs a quick lil' allocation performance test.
//

use std::time::Instant;

use stackvec::StackVec;


/***** BENCHMARK *****/
fn benchmark<const LEN: usize, const ITERS: usize, T>() {
    // Benchmark the vector
    let start: Instant = Instant::now();
    for _ in 0..ITERS {
        let _: Vec<u8> = core::hint::black_box(Vec::with_capacity(LEN));
    }
    let vec_time: u128 = start.elapsed().as_nanos();

    // Benchmark the stackvec
    let start: Instant = Instant::now();
    for _ in 0..ITERS {
        let _: StackVec<LEN, u8> = core::hint::black_box(StackVec::new());
    }
    let stack_time: u128 = start.elapsed().as_nanos();

    // Print the result
    println!(
        "{} {}\n > Vec = {}ns/{}iters = {}ns/iter\n > StackVec = {}ns/{}iters = {}ns/iter (speedup {}x)",
        LEN,
        std::any::type_name::<T>(),
        vec_time,
        ITERS,
        vec_time as f64 / ITERS as f64,
        stack_time,
        ITERS,
        stack_time as f64 / ITERS as f64,
        vec_time as f64 / stack_time as f64
    );
}





/***** ENTRYPOINT *****/
fn main() {
    benchmark::<100, 1000000000, u8>();
    benchmark::<1000, 1000000000, u8>();
    benchmark::<10000, 1000000000, u8>();
    benchmark::<100, 1000000000, u32>();
    benchmark::<1000, 1000000000, u32>();
    benchmark::<10000, 1000000000, u32>();
    benchmark::<100, 1000000000, String>();
    benchmark::<1000, 1000000000, String>();
    benchmark::<10000, 1000000000, String>();
}
