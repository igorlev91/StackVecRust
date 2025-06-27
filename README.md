# stackvec-rs
Implements an alterative to a Rust `Vec` that lives on the stack instead of on the heap, making creating it much faster.

The main type contributed is the `StackVec`, which has a [`Vec`](https://doc.rust-lang.org/std/vec/struct.Vec.html)-like interface except for usage on the stack.

For example:
```rust
use stackvec::StackVec;

// Allocates space on the stack for 20 elements
let mut vec: StackVec<3, &str> = StackVec::new();
vec.push(" > Hello there!");
vec.push(" > General Kenobi, you are a bold one");
vec.push(" > Back away, I will deal with this Jedi slime myself");
println!("{}", vec.join("\n"));
```

If you allocate more than the reserved space, it does not automatically re-allocate like a `Vec` because it lives on the heap. Instead, it will panic:
```rust
use stackvec::StackVec;

// Allocates space on the stack for 20 elements
let mut vec: StackVec<3, &str> = StackVec::new();
vec.push("foo");
vec.push("bar");
vec.push("baz");
vec.push("quz");   // Panic: Cannot push 4th element to StackVec of capacity 3
```

Loosing this flexibility does mean a performance benefit because no allocation is needed:
| Type   | Capacity (#elements) | Vec Avg. Time (ns) | StackVec Avg. Time (ns) | Speedup (ratio)    |
|--------|----------------------|--------------------|-------------------------|--------------------|
| u8     | 100                  | 8.4345681          | 0.356255401             | 23.675621692539618 |
| u8     | 1000                 | 8.674978622        | 0.476285096             | 18.213835987847077 |
| u8     | 10000                | 20.701679843       | 0.351451929             | 58.90330407889154  |
| u32    | 100                  | 8.915992469        | 0.348230987             | 25.603673428981782 |
| u32    | 1000                 | 8.764270772        | 0.378424042             | 23.159920616248794 |
| u32    | 10000                | 21.503726902       | 0.48956447              | 43.92419838392276  |
| String | 100                  | 9.059891424        | 0.383766724             | 23.607808747899675 |
| String | 1000                 | 9.550490999        | 0.489552548             | 19.508612585139687 |
| String | 10000                | 21.243622152       | 0.388852937             | 54.63150752028395  |

(Benchmarks are averaged over 1,000,000,000 attempts with `--release` on an AMD Ryzen 7 PRO 5850U. See [examples/benchmark.rs](./examples/benchmark.rs).)


## Installation
To use the crate, simply add it as a dependency to your [`Cargo.toml`](https://doc.rust-lang.org/cargo/reference/manifest.html):
```toml
[dependencies]
stackvec = { git = "https://github.com/igorlev91/StackVecRust" }
```

You can also commit to a specific tag by supplying it:
```toml
[dependencies]
stackvec = { git = "https://github.com/igorlev91/StackVecRust", tag = "v0.2.0" }
```


## Usage
To inspect the code documentation, either use [`rust-analyzer`] to integrate the crate's documents into your IDE, or generate a standalone HTML:
```bash
cargo doc --no-deps --open
```



## License
This project is licensed under GPLv3. See [LICENSE](./LICENSE) for more information.
