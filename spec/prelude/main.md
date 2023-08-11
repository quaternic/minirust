# MiniRust prelude

Across all files in this repository, we assume some definitions to always be in scope.

```rust
/// Documentation for libspecr can be found here: https://docs.rs/libspecr
pub use libspecr::prelude::*;
// pub use Align as defined below, to shadow the one from libspecr::prelude
pub use self::align_with_remainder::Align;
pub use self::align_with_remainder::is_aligned_for;


/// Make the two main modules available.
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


mod align_with_remainder {
    use super::Int;
    use super::Size;
    use std::cmp::Ordering;

    /// Generalized alignment:
    ///     r mod d
    /// Represents the set of integers k * d + r, for all integers k
    #[derive(Copy,Clone,PartialEq,Eq,Hash,Debug,GcCompat)]
    pub struct Align {
        d: D,
        r: Int,
    }

    /// Nonnegative integers ordered by divisibility:
    ///         4 < ...
    ///     2 <        ...
    /// 1 <     6 < ...   ... < 0
    ///     3 <        ...
    ///         9 < ...
    /// For this purpose it is convenient to consider zero as being divisible by
    /// anything, including 0, with the rationale that the set k * 0 + r is just
    /// the singleton set {r} which is a subset of any set {forall k: r + k * d}.
    #[derive(Copy,Clone,Debug,GcCompat)]
    struct D(Int);

    impl std::cmp::PartialOrd for D {
        fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
            let a = self.0;
            let b = rhs.0;

            match (a == 0, b == 0) {
                // Both nonzero, examine division
                (false, false) => {
                    let a_is_kb = a % b == 0;
                    let b_is_ka = b % a == 0;
                    // If either is a multiple of the other, it compares greater.
                    // Otherwise None.
                    (a_is_kb | b_is_ka).then_some(a_is_kb.cmp(&b_is_ka))
                }
                // Otherwise the one that is zero compares greater (or equal)
                (a_zero, b_zero) => Some(a_zero.cmp(&b_zero)),
            }
        }
    }

    /// Greatest common divisor
    /// If either input is 0, returns the other input, or 0 if both are
    fn gcd(D(mut x): D, D(mut y): D) -> D {
        while y != Int::ZERO {
            (x,y) = (y, x % y);
        }
        D(x)
    }

    impl std::cmp::PartialOrd for Align {
        fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
            // Compare the divisors by divisibility
            self.d.partial_cmp(&rhs.d)
                .filter(|order| match order {
                    // Any ordering additionally requires that the remainders
                    // are congruent modulo the smaller divisor.
                    Ordering::Equal   => self.r == rhs.r,
                    Ordering::Less    => self.d <= D(self.r - rhs.r),
                    Ordering::Greater => rhs.d  <= D(self.r - rhs.r),
                })
        }
    }

    // FIXME: could be a method on Int, or some Address(Int)-style wrapper
    pub fn is_aligned_for(addr: Int, align: Align) -> bool {
        align.d <= D(align.r - addr)
    }

    /// Alignments add together to an Align representing all the possible
    /// pairwise sums.
    /// E.g. (3 mod 15) + (8 mod 21) == (2 mod 3)
    impl std::ops::Add for Align {
        type Output = Self;
        fn add(mut self, rhs: Self) -> Self {
            // The largest alignment that divides both
            self.d = gcd(self.d, rhs.d);
            // Add the remainders modulo the new divisor
            self.r += rhs.r;
            self.remainder_reduction();
            self
        }
    }

    impl Align {
        pub const ONE: Self = Self {
            r: Int::ZERO,
            d: D(Int::ONE),
        };

        /// Returns the fixed offset as an alignment
        pub fn from_offset(bytes: Int) -> Self {
            Self { d: D(Int::ZERO), r: bytes }
        }

        /// Returns the Align representing all multiples of `size`
        pub fn from_stride(size: Int) -> Self {
            Self { d: D(size.into()), r: Int::ZERO }
        }

        pub const fn from_bits_const(bits: usize) -> Option<Self> {
            if bits.is_power_of_two() && bits >= 8 {
                let bytes = bits / 8;
                Some(Self { d: D(Int::const_from(bytes)), r: Int::ZERO })
            } else {
                panic!("number of bits is not a suitable power of two")
            }
        }

        // FIXME: this is a hack to avoid needing to change various
        // callsites that just use it to get something printable
        pub fn bytes(self) -> impl std::fmt::Display {
            assert!(self.d.0.is_power_of_two());
            assert!(self.r == Int::ZERO);
            self.d.0
        }

        // FIXME: Temporary helper to avoid exposing implementation details just
        // to get a distribution that produces suitable addresses
        pub fn pick(self, start: Int, end: Int, f: impl Fn(Int) -> bool) -> super::NdResult<Int> {
            let r = self.r;
            let (start, end, divisor) = if self.d.0 == Int::ZERO {
                (Int::ZERO, Int::ONE, Int::ONE)
            } else {
                (start - r, end - r, self.d.0)
            };
            let distr = libspecr::IntDistribution { start, end, divisor };
            super::ret(super::pick(distr, |x| f(x + r))? + r)
        }

        // If the offset has been
        fn remainder_reduction(&mut self) {
            if self.d.0 != 0 {
                // to compensate for the lack of an modulo-operation that would
                // always output a value in 0 <= r < d
                self.r %= self.d.0;
                if self.r < 0 {
                    self.r += self.d.0.abs();
                }
                assert!(self.r >= 0);
                assert!(self.r < self.d.0.abs());
            }
        }
    }



    // Boilerplate necessary for D(x) == D(-x)
    // Could instead normalize sign on construction, but D is internal to this
    // module and mainly used as a thin wrapper for its partial order impl
    impl std::cmp::PartialEq for D {
        fn eq(&self, rhs: &Self) -> bool {
            self.0.abs() == rhs.0.abs()
        }
    }
    impl std::cmp::Eq for D { }
    impl std::hash::Hash for D {
        fn hash<H>(&self, hasher: &mut H) where H: std::hash::Hasher {
            self.0.abs().hash(hasher);
        }
    }

}

```
