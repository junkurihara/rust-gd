# rust-gd

Generalized Deduplication based on error-correcting codes, written in Rust.

## Current Status

Currently, the very basic of Generalized Deduplication algorithm is being implemented, which is a very novel "deduplication" algorithm using the idea of sphere packing in the context of error-correcting codes.

> - Vestergaard, Rasmus, Qi Zhang, and Daniel E. Lucani. "Generalized deduplication: bounds, convergence, and asymptotic properties." 2019 IEEE Global Communications Conference (GLOBECOM). IEEE, 2019.
> - Vestergaard, Rasmus, Daniel E. Lucani, and Qi Zhang. "Generalized deduplication: Lossless compression for large amounts of small IoT data." European Wireless 2019; 25th European Wireless Conference. VDE, 2019.
> - etc...

Our implementation is based on:

- Hamming codes of `(7, 4)`, `(15, 11)`, `(31, 26)`, `(63, 57)`, `(127, 120)`, `(255, 247)` ...

- (TODO) Deletion and deviation using PRNG (Yggdrasil paper)

- (TODO) Reed-Solomon codes over `GF(256)` (not yet)
  - I am not sure how RS code works. This is because RS is NOT perfect, in the sense of sphere packing. So, the technique to calculate the `base` = RS codeword and `dev` = error from arbitrary byte sequence has to require some trick.
  - I guess in RS codes, first find a base(s) near the given byte sequence `S` in terms of the Hamming distance, fix one base `B`, and compress the difference `D = S + B`, where `D` can be shortened since it has few numbers of non-zero bytes.

We note that `(7,4)` Hamming code doesn't work in the GD, the internal `libecc` supports although. This is because the message length, i.e., of length 4 bits, is not sufficient to encode a "byte", i.e., of length 8 bits. Our implementation employs an encoding method that chunks message sequences in the unit of bytes. For example, if `(15, 11)` Hamming code is employed, 23-bit messages are divided into two 8 bits sequences and one 7 bit sequences, and pads 3 zeros to the first seqs and 4 zeros to the last one to deal as message sequences.
