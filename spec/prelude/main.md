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
    /// Generalized alignment:
    ///     x mod (1<<k)
    /// Represented as the nonzero integer:
    ///     (1<<k) + x % (1<<k)
    ///
    // The highest set bit in Align marks the modulus (a power-of-two),
    // and all the less significant bits form the remainder which is always
    // less than the modulus.
    #[derive(Copy,Clone,PartialEq,Eq,Hash,Debug,GcCompat)]
    pub struct Align(Int);

    // TODO: Align(0) is currently an invalid value. It might instead be used to
    // mean an unsatisfiable alignment.

    use std::cmp::Ordering;
    impl std::cmp::PartialOrd for Align {
        /// Partial order by fixed low bits:
        ///
        /// An alignment with remainder is "less than" another iff it requires at
        /// most as many fixed low bits as the other, and those have the same value.
        ///
        /// (2 mod 4) is "less than" either (2 mod 8) and (6 mod 8), but the latter
        /// two are incomparable, and there is no value greater than both
        ///
        /// At the low end, (0 mod 1) == 0u1 == Align::ONE is less than any
        /// other, so the full relation has the structure of a binary tree:
        /// (with shorthand RuN to mean R mod 2.pow(N), as if you had a dynamic-
        /// width version of the normal wrapping integer types)
        ///
        ///                 < 0u4 < ...
        ///           < 0u3 < 4u4 < ...
        ///         /       < 2u4 < ...
        ///     < 0u2 < 2u3 < 6u4 < ...
        /// 0u1
        ///     < 1u2 < 1u3 < 1u4 < ...
        ///         \       < 5u4 < ...
        ///           < 3u3 < 3u4 < ...
        ///                 < 7u4 < ...
        ///
        /// That is, the Align that requires an address like  ...xyz, compares
        /// "less than" to all the Aligns requiring ...w0xyz and ...w1xyz for
        /// any bit-string w. In the other direction,  ...xyz is "greater than"
        /// ...yz, ...z, and ...
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            // First compare as integer, equality means equality, and otherwise
            // it gives the only possible ordering.
            let cmp = self.0.cmp(&other.0);
            if cmp.is_eq() {
                return Some(Ordering::Equal);
            }
            // The rest of it decides between None and Some

            let (min, max) = match cmp {
                Ordering::Equal => return Some(Ordering::Equal),
                Ordering::Less => (self.0,other.0),
                Ordering::Greater => (other.0,self.0),
            };
            let absdiff = max - min;

            // Since the smaller of the two needs to have the form ...0001xyz,
            // where xyz are also the last bits of the other term, we can
            // simply shift the smaller number down by the count of equal low
            // bits plus 1, and check if that shifts out all set bits. The
            // count of trailing zeroes may be larger by coincidence, but it
            // doesn't affect the result.
            let same_bits = absdiff.trailing_zeros().unwrap();
            if min >> (same_bits + 1) == Int::ZERO {
                Some(cmp)
            } else {
                None
            }
        }
    }
    // FIXME: could be a method on Int, or some Address(Int)-style wrapper
    pub fn is_aligned_for(addr: Int, align: Align) -> bool {
        //FIXME do properly
        let next = align.0.next_power_of_two();
        // adding the required alignment, or twice that, doesn't change
        // whether the address is aligned for the requirement, but it ensures
        // that the number makes sense as the inner representation for Align
        align <= Align(addr + next)
    }

    // helper that could be a method of Int
    fn find_highest_set_bit(x: Int) -> Int {
        let next = x.next_power_of_two();
        let tz = next.trailing_zeros().unwrap();
        if next == x {
            tz
        } else {
            tz - 1
        }
    }

    impl Align {
        pub const ONE: Self = Self(Int::ONE);

        /// Creates an Align from a power-of-two count of bytes
        /// Panics if called with bytes = 0, and if it is something else
        /// and not a power-of-two, returns None
        pub fn from_bytes(bytes: impl Into<Int>) -> Option<Self> {
            let bytes = bytes.into();
            if bytes == Int::ZERO {
                panic!("invalid argument in Align::from_bytes({bytes:?})")
            } else if bytes.is_power_of_two() {
                Some(Self(bytes))
            } else {
                None
            }
        }
        pub fn modulus(self) -> Int {
            // What we actually want is rounding down to a power of two
            let mut next = self.0.next_power_of_two();
            if next != self.0 {
                next >>= 1;
            }
            next
        }
        pub fn remainder(self) -> Int {
            self.0 - self.modulus()
        }
        pub fn is_normal(self) -> bool {
            self.0.is_power_of_two()
        }
        /// The highest alignment of the current remainder
        pub fn reduced(self) -> Align {
            let least = self.0.trailing_zeros().unwrap();
            Align(Int::ONE << least)
        }
        // FIXME: this is a hack to avoid needing to change various
        // callsites that just use it to get something printable
        pub fn bytes(self) -> impl std::fmt::Display {
            assert!(self.is_normal());
            self.modulus()
        }
        pub const fn from_bits_const(bits: usize) -> Option<Self> {
            if bits.is_power_of_two() {
                let bytes = bits / 8;
                assert!(bytes != 0);
                Some(Self(Int::const_from(bytes)))
            } else {
                None
            }
        }
        /// Adjusts the remainder for a fixed offset
        pub fn constant_offset(self, offset: Size) -> Self {
            // Example: Align(21) means 5 (mod 16)
            // Align(21).constant_offset(25) means (5 + 25) == 14 (mod 16)
            let m = self.modulus();
            let r = (self.0 + offset.bytes()) & (m - 1);
            Self(m + r)
        }
        /// The new alignment after adding an unknown multiple of a fixed offset.
        ///
        /// Picks the greatest possible alignment that does not change with the
        /// index, and restricts the remainder to that.
        pub fn indexed_offset(self, element_size: Size) -> Self {
            if element_size.is_zero() {
                // indexing ZSTs doesn't change the alignment at all
                self
            } else {
                // Example: Align(21).indexed_offset(12) means (5 + k*12)
                // where k is an unknown integer so the result is the common alignment
                // of 5, 17, 29, 41, ... == 1 (mod 4)
                let m = find_highest_set_bit(self.0).min(element_size.bytes().trailing_zeros().unwrap());
                let m = Int::ONE << m;
                let r = self.0 & (m - 1);
                Self(m + r)
            }
        }

    }
}

```
