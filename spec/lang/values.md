# MiniRust Values

The purpose of this file is to describe what the set of *all possible values* is in MiniRust.
Basic operations such as conditionals and arithmetic act on these values.

```rust
enum Value<M: Memory> {
    /// A mathematical integer, used for `i*`/`u*` types.
    Int(Int),
    /// A Boolean value, used for `bool`.
    Bool(bool),
    /// A pointer value, used for (thin) references and raw pointers.
    Ptr(Pointer<M::Provenance>),
    /// An n-tuple, used for arrays, structs, tuples (including unit).
    Tuple(List<Value<M>>),
    /// A variant of a sum type, used for enums.
    Variant {
        idx: Int,
        #[specr::indirection]
        data: Value<M>,
    },
    /// Unions are represented as "lists of chunks", where each chunk is just a raw list of bytes.
    Union(List<List<AbstractByte<M::Provenance>>>),
}
```

The point of this type is to capture the mathematical concepts that are represented by the data we store in memory.
The definition is likely incomplete, and even if it was complete now, we might expand it as Rust grows.
That is okay; all previously defined representation relations are still well-defined when the domain grows, the newly added values will just not be valid for old types as one would expect.
