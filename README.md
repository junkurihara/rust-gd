# rust-gd: A Rust implementation of Generalized Deduplication

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
    "寿限無(じゅげむ)寿限無(じゅげむ)五劫(ごこう)のすりきれ海砂利(かいじゃり)padpadpadpadpadpadpadpad".to_string().as_bytes()

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

### GD with Hamming code

```rust:
let hamming_deg = 4;         // Degree m of (2^m - 1, 2^m - m -1) Hamming code
let hamming_dict_size = 511; // max entry size of a dictionary used in GD process

let to_be_deduped: &[u8] =
    "寿限無(じゅげむ)寿限無(じゅげむ)五劫(ごこう)のすりきれ海砂利(かいじゃり)padpadpadpadpadpadpadpad".to_string().as_bytes()

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

Currently, our implementation is based on Hamming code and Reed-Solomon code

### Hamming codes of `(2^m - 1, 2^m - m - 1)` for degree `m = 4, ..., 11`

In the internal `libecc` library of error-correcting codes, Hamming code with `m = 3` works. However, the code length, i.e., 7 bits, is insufficient to represent even a byte. In order to reasonably deduplicate byte-based data, *byte alignment* is needed. So, we omitted this small length parameter.

### Reed-Solomon codes over `GF(256)`

Unlike Hamming codes, RS codes are **non-perfect* codes, which means the simple *sphere packing* approach cannot be directly applied to employ GD. This implies that for an arbitrary given chunk `c = [c_1,...,c_n] in GF(256)^n`, the following does **NOT** always hold in an `(n, k)` RS code `C`:

```
|argmin { d_H(c, v) : v in C }| = 1 (necessary condition of perfect codes)
```

where `d_H(c, v)` is the Hamming distance between `c` and `v`. In other words, there may exist more than one codeword nearest to the chunk. If it is unique, an arbitrary chunk `c` can be uniquely converted to a tuple of (a codeword `cw` as a *base*, a virtual additive error `er` as *deviation*), i.e., `c = cw + er`. However, it doesn't hold for RS codes. Thus for RS codes, we need a certain rule to *forcibly and deterministically* map a given chunk to a codeword even if there exist two ore more candidates of such codeword.

To this end, we take the following rule in our implementation.

---

### 1. Simple split of given chunks

Let `c = [c_1,...c_n] in GF(256)^n` be a given data chunk, and `c_i` be a byte, i.e., an element over `GF(256)`. We assume `(n, k)` RS code `C` is employed.

In our implementation, `c` is first simply split into two subvectors:

```
cl = [c_1,c_2,...c_k] in GF(256)^k,
cr = [c_{k+1},...c_n] in GF(256)^{n-k},
```

We shall regard this `cl` as equivalent to the base `cw in C`, that is, `cw = G(cl)` for a certain bijective mapping `G: GF(256)^k -> C`.

We also regard `cr` as equivalent to the deviation `er` satisfying `c = cw + er`.

### 2. Assumption of systematic codes

The bijection `G` can be easily obtained as a *systematic generator matrix* `G = [I | P]`, where `I` is an identity matrix. Namely, the base `cw` and `er` is obtained as follows.

```
cw = cl G
   = cl [I | P]
   = [cl, cl P],
er = [0,...,0, cl P + cr],
```

### 3. Base and deviation in deduplicated output and dictionary

In deduplicated data stream and dictionary in deduplication instance, we suppose that the base is simply given by `cl in GF(256)^k` and the deviation is `cl P + cr in GF(256)^{n-k}`.

---

### Rationale and more reasonable approach for generic data

Observe that in GD, we can arbitrarily assume *positions of virtual additive errors* in a given chunk to calculate the deviation. In the above method, we simply suppose that the message part `cl = [c_1,...,c_k]` of chunk `c` is error-free. Thus, we can uniquely fix the base, i.e., `cw = cl G ~ cl`, and the deviation `er = [0,...,0, cl P + cr] ~ cl P + cr` as well. Thus we can execute GD by applying this constraint.

However, *since virtual additive errors are fluctuations of given data, they would not always be the last `n-k` symbols of an `n` byte chunk `c`* even if we carefully choose parameters `n` and `k` according to the given data type. Thus, in order to reasonably apply an `(n,k)` RS code for a specific data type in GD, **we should also configure the positions of virtual additive errors as well.**

To this end, we can take an approach of the *error alignment* or *pushing errors aside* on given chunk by precoding the data. In particular according to a specific data type, we first fix a linear transformation `T: GF(256)^n -> GF(256)^n`, i.e., a nonsingular `n x n` matrix `T` over `GF(256)`, where `T` is typically a simple permutation matrix. We then execute the precoding on a given chunk `c` as follows.

```
[xl, xr] = cT, where xl in GF(256)^k and xr in GF(256)^{n-k}
```

Then, the base `cw` and the deviation `er` are calculated on `[xl, xr]` instead of `[cl, cr]` by the above approach, as follows:

```
cw = xl G = [xl, xl P],
er = [0,...,0, xl P + xr],
```

and `xl` is recoded as a base and `xl P + xr` is regarded as a deviation in a deduplicated data stream and the GD dictionary.

We should note that *the linear transformation `T` pushes the virtual errors contained in `c` of the specific data form to the last `n-k` symbols of a transformed chunk of `n` symbols.*


## TODO

Following should be considered to be implemented.

- Deletion and deviation using PRNG (Yggdrasil paper)

- Golomb-Rice codes

## Caveats

We note that `(7,4)` Hamming code doesn't work in our implementation, the internal `libecc` supports although. This is because the code length, i.e., 7 bits, is not sufficient to deduplicate a "byte", i.e., of length 8 bits. Our implementation employs an encoding method that chunks message sequences in the unit of bytes. For example, if `(15, 11)` Hamming code is employed, a 2-bytes message is divided into two one byte (= 8 bits) sequences, and pads 7 bits of zeros to each sequence to deal as 15-bits codeword of Hamming code.

## License

Licensed under the MIT license, see `LICENSE` file.
