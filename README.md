# rust-gd

Generalized Deduplication based on error-correcting codes, written in Rust.

## Current Status

Currently, the very basic of Generalized Deduplication algorithm is being implemented, which is a very novel "deduplication" algorithm using the idea of sphere packing in the context of error-correcting codes.

> - Vestergaard, Rasmus, Qi Zhang, and Daniel E. Lucani. "Generalized deduplication: bounds, convergence, and asymptotic properties." 2019 IEEE Global Communications Conference (GLOBECOM). IEEE, 2019.
> - Vestergaard, Rasmus, Daniel E. Lucani, and Qi Zhang. "Generalized deduplication: Lossless compression for large amounts of small IoT data." European Wireless 2019; 25th European Wireless Conference. VDE, 2019.
> - etc...

Our implementation is based on:

- Hamming codes of `(2^m - 1, 2^m - m - 1)` for degree `m = 3, ..., 11`.

- Reed-Solomon codes over `GF(256)` (basic library was implemented, not usable to execute GD yet). In order to utilize RS code, which is *non-perfect* code, for GD, the following method is supposed.

  - Let `c = [x_1,...x_n]` be a data chunk, and `x_i` be a byte, i.e., an element over GF(256). Assume `(n, k)` RS code with a generator matrix `G = [I P]` is employed, where `I` is an identity matrix.
  - The given data chunk `c` is simply split into the following two parts, `b` and `z`.

    ```shell:
    b = [x_1, ..., x_k]
    d = [x_k+1, ..., x_n]
    ```

  - A codeword `w` is generated as `w = bG = [bI bP] = [b bP]`, and we have `d = bP + z`. Then finally `b` and `d` are treated as a base and a deviation in the context of GD.

  Note that RS code is *non-perfect* in terms of sphere packing (doesn't meet the Hamming bound). This means that for a given chunk, we cannot uniquely determine one nearest codeword (may exist multiple codewords). Hence we need a trick to uniquely map a given arbitrary chunk `c` of length `n` to a codeword (base) `w` of an `(n, k)` RS code and additive error `d`(deviation), i.e., `c = w + d`.

  Observe that in GD, we can arbitrarily assume *positions of virtual errors* in a given chunk to calculate the deviation. In the above method, we supposed that the message part, i.e., `[x_1,...,x_k]`, of the chunk is error-free. Thus, we can uniquely fix the base, i.e., `b ~ w = bG`, and the deviation `d ~ w + [0, d]` as well.

  The above strategy by splitting a chunk is really simple, and we may need more generalized approach that arbitrary splits vector space `GF(256)^n`. This can be simply done by fixing a linear transformation `T: GF(256)^n -> GF(256)^n`, i.e., a non-singular `n x n` matrix `T` over `GF(256)`, and calculate a base `b` and a deviation `d` for a given chunk `c` as

  ```shell:
  1. [x, y] = cT
  2. [b, xP]= x G = [x xP]
  3. d = xP + y
  ```

- (TODO) Deletion and deviation using PRNG (Yggdrasil paper)

- (TODO) Golomb-Rice codes

We note that `(7,4)` Hamming code doesn't work in the GD, the internal `libecc` supports although. This is because the message length, i.e., of length 4 bits, is not sufficient to encode a "byte", i.e., of length 8 bits. Our implementation employs an encoding method that chunks message sequences in the unit of bytes. For example, if `(15, 11)` Hamming code is employed, 23-bit messages are divided into two 8 bits sequences and one 7 bit sequences, and pads 3 zeros to the first seqs and 4 zeros to the last one to deal as message sequences.
