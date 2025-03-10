# MiniRust prelude

Across all files in this repository, we assume some definitions to always be in scope.

```rust
/// Documentation for libspecr can be found here: https://docs.rs/libspecr
pub use libspecr::prelude::*;

pub use crate::{lang, mem};

/// All operations are fallible, so they return `Result`.  If they fail, that
/// means the program caused UB or put the machine to a halt.
pub type Result<T=()> = std::result::Result<T, TerminationInfo>;

#[non_exhaustive]
pub enum TerminationInfo {
    /// The execution encountered undefined behaviour.
    Ub(String),
    /// The program was executed and the machine stopped without error.
    MachineStop,
    /// The program was ill-formed.
    IllFormed,
    /// The program did not terminate but no thread can make progress.
    Deadlock,
}

/// Some macros for convenient yeeting, i.e., return an error from a
/// `Option`/`Result`-returning function.
macro_rules! throw {
    ($($tt:tt)*) => {
        do yeet ()
    };
}
macro_rules! throw_ub {
    ($($tt:tt)*) => {
        do yeet TerminationInfo::Ub(format!($($tt)*))
    };
}
macro_rules! throw_machine_stop {
    () => {
        do yeet TerminationInfo::MachineStop
    };
}
macro_rules! throw_ill_formed {
    () => {
        do yeet TerminationInfo::IllFormed
    };
}
macro_rules! throw_deadlock {
    () => {
        do yeet TerminationInfo::Deadlock
    };
}

/// We leave the encoding of the non-determinism monad opaque.
pub use libspecr::Nondet;
pub type NdResult<T=()> = libspecr::NdResult<T, TerminationInfo>;
```
