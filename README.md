# lexvarint

`lexvarint` encodes unsigned integers as ASCII strings whose bytewise
lexicographic order matches their numeric order. The API covers every value
representable by `u128`.

Each encoding starts with a three-digit segment count, followed by that many
underscore-prefixed, three-digit, big-endian base-1000 segments:

```text
0    -> 000
1    -> 001_001
999  -> 001_999
1000 -> 002_001_000
```

```rust
let encoded = lexvarint::encode(1_000);
assert_eq!(encoded, "002_001_000");

let decoded = lexvarint::decode(&encoded)?;
assert_eq!(decoded, 1_000);
# Ok::<(), lexvarint::DecodeError>(())
```

The decoder accepts only canonical encodings. Values are ordered by raw bytes,
so databases must use binary collation for encoded keys.
