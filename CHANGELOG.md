# CHANGELOG
This file keeps track of changes to the `stackvec` Rust crate.

This project uses [semantic versioning](https://semver.org). As such, breaking changes are indicated with **\[breaking\]**.


## v0.3.0
### Added
- Some unit tests for (future) debugging purposes.
- Implementations for `PartialEq` for `StackVec` with `[T, LEN2]`, `&'a [T]` and `Vec<T>`.

### Changed
- `From<[T, LEN]>` is now implemented for `From<T, LEN2>` instead to also allow building it from arrays that don't consume the full capacity.
    - Note: not a breaking change because this is strictly more powerful than before.

### Fixed
- A double free error when iterating-by-ownership over a `StackVec` with `Drop`-elements.


## v0.2.0
### Added 
- A `Deref<Target = [u8]>` implementation for the `StackVec`.
    - This should give it a great deal of implementations for convenient slice operations.
    - **Note**: Some functions have now moved to the slice implementations. Should not be breaking, but be aware.
- A `From<StackVec>` implementation for `Vec`s.


## v0.1.0
Initial release!

### Added
- The `StackVec`, a `Vec`-like data structure that lives on the stack.
