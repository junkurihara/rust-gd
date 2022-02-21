# rust-gd: A Rust Implementation of Generalized Deduplication

Rust implementation of *Generalized Deduplication* (GD) based on several types of error-correcting codes.

This is an implementation (and somewhat extension) of the novel concept of data deduplication method, called *Generalized Deduplication* (GD). The original concept of GD was introduced by a group of Aarhus University, Denmark, leaded by [Prof. D. E. Lucani](https://pure.au.dk/portal/en/persons/daniel-enrique-lucani-roetter(c4e78b1e-4dd6-460f-9c44-1a44771ce01a).html).

> - Vestergaard, Rasmus, Qi Zhang, and Daniel E. Lucani. "Generalized deduplication: bounds, convergence, and asymptotic properties." 2019 IEEE Global Communications Conference (GLOBECOM). IEEE, 2019.
> - Vestergaard, Rasmus, Daniel E. Lucani, and Qi Zhang. "Generalized deduplication: Lossless compression for large amounts of small IoT data." European Wireless 2019; 25th European Wireless Conference. VDE, 2019.
> - etc.

## Usage

Add the following to your `Cargo.toml` as imported directly from GitHub:

```toml:Cargo.toml
[dependencies]
rust-gd = { git = "https://github.com/junkurihara/rust-gd.git" }
```

or from crates.io (not published yet):

```toml:Cargo.toml
[dependencies]
rust-gd = "0.1.0"
```

Then, add `use` in your `.rs` file.

```rust:
use rust_gd::*;
```

## Example

**NOTE: The compression rate strongly depends on the data alignment and data structure. So you should carefully choose the parameters according to the characteristics of given data**.

### GD with Reed-Solomon code over GF(256)

```rust:
use rust_gd::*;

let to_be_deduped: &[u8] =
    "寿限無(じゅげむ)寿限無(じゅげむ)五劫(ごこう)のすりきれ海砂利(かいじゃり)padpadpadpadpadpadpadpad".to_string().repeat(128).as_bytes()

let code_len = 128; // codeword length over GF(256), i.e., N in (N, K) RS code
let msg_len = 124;  // message length over GF(256), i.e., K in (N, K) RS code
let dict_size = 127; // max entry size of a dictionary used in GD process

// GD instance for deduplication (compress)
let mut gd_dedup = GD::ReedSolomon(code_len, msg_len).setup(dict_size).unwrap();

// GD instance for duplication (decompress)
let mut gd_dup = GD::ReedSolomon(code_len, msg_len).setup(dict_size).unwrap();

// struct Deduped = {pub data: Vec<u8>, pub last_chunk_pad_bytelen: usize}
let deduped: Deduped = gd_dedup.dedup(to_be_deduped).unwrap();
println!("> Deduped data size is {} bytes", x.data.len());

let duped: Vec<u8> = gd_dup.dup(&deduped).unwrap();
println!("> Duped size {} bytes", y.len();

assert_eq!(duped, words);
```

In GD with RS codes, **error-alignment** can be employed by

```rust:
// Linear transformation matrix used for error-alignment. This must be nonsinglar.
let trans: [&[u8; 4]; 4] = [
      &[1, 0, 0, 0],
      &[1, 1, 1, 4],
      &[1, 1, 3, 0],
      &[1, 2, 0, 0],
    ];

// Instantiation
let mut gd_dedup = GD::ReedSolomon(4, 3).setup(15).unwrap();
let mut gd_dup = GD::ReedSolomon(4, 3).setup(15).unwrap();

// Set error alignment
let res_dedup = gd_dedup.set_error_alignment(trans); // this simply returns Result<()>
let res_dup = gd_dup.set_error_alignment(trans);   // this simply returns Result<()>
assert!(res_dedup.is_ok());
assert!(res_dup.is_ok());

// then use gd instances to deduplicate/duplicate data as above.
```

For the detailed design of RS-code based implementation and the basic idea error-alignment, see [DESIGN.md](./DESIGN.md).

### GD with Hamming code

```rust:
let hamming_deg = 4;         // Degree m of (2^m - 1, 2^m - m -1) Hamming code
let hamming_dict_size = 511; // max entry size of a dictionary used in GD process

let to_be_deduped: &[u8] =
    "寿限無(じゅげむ)寿限無(じゅげむ)五劫(ごこう)のすりきれ海砂利(かいじゃり)padpadpadpadpadpadpadpad".to_string().repeat(128).as_bytes()

// GD instance for deduplication (compress)
let mut gd_dedup = GD::Hamming(hamming_deg).setup(hamming_dict_size).unwrap();

// GD instance for duplication (decompress)
let mut gd_dup = GD::Hamming(hamming_deg).setup(hamming_dict_size).unwrap();

// struct Deduped = {pub data: Vec<u8>, pub last_chunk_pad_bytelen: usize}
let deduped: Deduped = gd_dedup.dedup(to_be_deduped).unwrap();
println!("> Deduped data size is {} bytes", x.data.len());

let duped: Vec<u8> = gd_dup.dup(&deduped).unwrap();
println!("> Duped size {} bytes", y.len();
```

## Codes in our implementation

Currently, our implementation is based on Hamming code and Reed-Solomon (RS) code. GD based on RS codes processes data chunks as *byte stream*. On the other hand, Hamming-based GD considered as data chunks as *bit stream*.

For GD implementation based on Hamming codes, in the internal `libecc` library of error-correcting codes, Hamming code with `m = 3` works. However, the parameter of `m = 3` does not work in GD. This is because the code length, i.e., 7 bits, is not sufficient to deduplicate a "byte"-based data. In order to reasonably deduplicate byte-based data, *byte alignment* is needed. So, we omitted this small length parameter.

**Byte alignment**: Our implementation employs an encoding method that chunks message sequences in the unit of bytes. For example, if `(15, 11)` Hamming code is employed, a 2-bytes message is divided into two one byte (= 8 bits) sequences, and pads 7 bits of zeros to each sequence to deal as 15-bits codeword of Hamming code.

## TODO

Following should be considered to be implemented.

- RS codes with precoding option for **error alignment**

- Deletion and deviation using PRNG (Yggdrasil paper)

- Golomb-Rice codes

## Caveats

At this time this solution should be considered suitable for research and experimentation, further code and security review is needed before utilization in a production application.

## License

Licensed under the MIT license, see `LICENSE` file.
