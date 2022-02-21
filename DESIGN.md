
# Design of GD based on Reed-Solomon codes over `GF(256)`

Unlike Hamming codes, RS codes are **non-perfect* codes, which means the simple *sphere packing* approach cannot be directly applied to employ GD. This implies that for an arbitrary given chunk `c = [c_1,...,c_n] in GF(256)^n`, the following does **NOT** always hold in an `(n, k)` RS code `C`:

```
|argmin { d_H(c, v) : v in C }| = 1 (necessary condition of perfect codes)
```

where `d_H(c, v)` is the Hamming distance between `c` and `v`. In other words, there may exist more than one codeword nearest to the chunk. If it is unique, an arbitrary chunk `c` can be uniquely converted to a tuple of (a codeword `cw` as a *base*, a virtual additive error `er` as *deviation*), i.e., `c = cw + er`. However, it doesn't hold for RS codes. Thus for RS codes, we need a certain rule to *forcibly and deterministically* map a given chunk to a codeword even if there exist two ore more candidates of such codeword.

To this end, we take the following rule in our implementation.

---

## 1. Simple split of given chunks, and assumption on virtual error location

Let `c = [c_1,...c_n] in GF(256)^n` be a given data chunk, and `c_i` be a byte, i.e., an element over `GF(256)`. We assume `(n, k)` RS code `C` is employed.

In our implementation, `c` is first simply split into two subvectors:

```
cl = [c_1,c_2,...c_k] in GF(256)^k,
cr = [c_{k+1},...c_n] in GF(256)^{n-k},
```

Here we regard `cr` as a part containing errors and `cl` as error-free. *we fix virtual error locations at the last `n-k` symbols of a given chunk*.

## 2. Derivation of a corresponding codeword of `C` only from the first `k` symbols of a chunk `c`

We see that `(n,k)` RS codes are maximum distance separable and hence a codeword can be reconstructed at least its error-free `k` symbols of any positions. Thus, as we fixed virtual error positions above, i.e., `cr`, we can identify `cl` as a codeword (i.e., base) `cw in C`.

In the deduplication process, we then obtain a unique codeword `cw` corresponding to `c` from `cl`, that is, we have `cw = G(cl)` for a certain bijective mapping `G: GF(256)^k -> C`. Here, we suppose this bijection is expressed by a *systematic generator matrix* `G = [I | P]` of `C`. Namely, we have a codeword `cw` as follows:

```
cw = cl G
   = cl [I | P]
   = [cl, cl P].
```

For the codeword `cw`, the deviation, i.e., virtual additive error, `er` is easily computed in such a way that `c = cw + er`.

```
er = [0,...,0, cl P + cr],
```

## 3. Representation of base and deviation in deduplicated output and dictionary

In deduplicated data and dictionary in deduplication instances, the base `cw` and deviation `er` are expressed as `cl in GF(256)^k` and `cl P + cr in GF(256)^{n-k}`, respectively. This is because `cl` can be identified as `cw` as we mentioned. Also `cl P + cr` is identified as `er` from its special structure due to the systematic generator matrix and fixed positions of virtual errors.

---

## Rationale and more reasonable approach for generic data based on *error-alignment*

Observe that in GD, we can arbitrarily assume *positions of virtual additive errors* in a given chunk to calculate the deviation. In the above method, we simply suppose that the message part `cl = [c_1,...,c_k]` of chunk `c` is error-free. Thus, we can uniquely fix the base, i.e., `cw = cl G ~ cl`, and the deviation `er = [0,...,0, cl P + cr] ~ cl P + cr` as well. Thus we can execute GD by applying this constraint.

However, *since virtual additive errors are fluctuations in given data for its centered base, they would not always be contained at the last `n-k` symbols of an `n` byte chunk `c`* even if we carefully choose parameters `n` and `k` according to the given data type. Thus, in order to reasonably apply an `(n,k)` RS code for more generic data type in GD, **we should also configure the positions of virtual additive errors as well.**

To this end, we can also take an approach of the *"error alignment"* or *"pushing errors aside"* on given chunk by precoding data chunks (by applying `GD.align_error(m: Vec<Vec<u8>>)` method).

The very basic idea of error-alignment is given in the following paper in terms of *reordering high entropy bits*:

> Vestergaard, Rasmus, Daniel E. Lucani, and Qi Zhang. "Generalized deduplication: Lossless compression for large amounts of small IoT data." European Wireless 2019; 25th European Wireless Conference. VDE, 2019.

In our concept, the idea is a bit more generalized by employing *lienar transformation* instead of reordering (permutation). In particular for a specific data type, we first fix a linear transformation `T: GF(256)^n -> GF(256)^n`, i.e., a nonsingular `n x n` matrix `T` over `GF(256)`. Note that the simplest `T` is typically a simple permutation matrix to align error symbols to the last positions, as given in the above paper. We then execute the precoding on a given chunk `c` as follows.

```
[xl, xr] = cT,
```

where `xl in GF(256)^k` and `xr in GF(256)^{n-k}`. Then, the base `cw` and the deviation `er` are calculated on `[xl, xr]` instead of `[cl, cr]` by the above approach, as follows:

```
cw = xl G
   = [xl, xl P],
er = [0,...,0, xl P + xr],
```

and `xl` is recoded as a base and `xl P + xr` is regarded as a deviation in a deduplicated data stream and the GD dictionary.

We should note that *the linear transformation `T` pushes the virtual errors contained in `c` of the specific data form to the last `n-k` symbols of a transformed chunk of `n` symbols.*

The above operations are simply concatenated into the following:

```
T= [Tl | Tr]
cw = c Tl G = [c Tl, c Tl P]
er = [0,...,0, c Tl P + c Tr]
```

Since it is known that we need to properly configure virtual error positions to achieve better deduplication performance, code length and error-positions have been considered to be dynamically adjusted by splitting a chunk into subchunks forming a specific pattern of fluctuationss. In contrast, the error-alignment approach simply align errors in data chunk to the last positions, and a data chunk is processed by a single GD instance with single code parameter.

Anyways, **the most important factor to achieve better deduplication rate in GD is the estimation of fluctuation/virtual-error patterns contained in given data chunks**.
