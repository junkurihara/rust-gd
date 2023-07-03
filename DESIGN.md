
# Design of GD based on Reed-Solomon codes over $\mathrm{GF}(2^8)$

Unlike Hamming codes, RS codes are **non-perfect* codes, which means the simple *sphere packing* approach cannot be directly applied to employ GD. This implies that for an arbitrary given chunk $c = [c_1,\dots,c_n] \in \mathrm{GF}(q)^n$, the following does **NOT** always hold in an $(n, k)$ RS code $\mathcal{C}$ [^rs].

**The necessary condition of perfect codes:**

$$\left|\argmin \{ d_H(c, v) : v \in \mathcal{C} \}\right| = 1,$$

where $d_H(c, v)$ is the Hamming distance between $c \in \mathrm{GF}(q)^n$ and $v\in \mathrm{GF}(q)^n$. In other words, there may exist more than one codeword nearest to the chunk. If it is unique, an arbitrary chunk $c\in \mathrm{GF}(q)^n$ can be uniquely converted to a tuple of (a codeword $w \in \mathcal{C}$ as a *base*, a virtual additive error $e \in \mathrm{GF}(q)^n$ as *deviation*), i.e., $c = w + e$. However, it doesn't always hold for RS codes. Thus for RS codes, we need a certain rule to *forcibly and deterministically* map a given chunk to a codeword even if there exist two ore more candidates of such codewords.

[^rs]: an $(n,k)$ linear code $\mathcal{C}$ over $\mathrm{GF}(q)$ means a linear subspace $C \subseteq \mathrm{GF}(q)^n$ of dimension $k$.

To this end, we take the following rule in our implementation.

---

## 1. Simple split of given chunks, and assumption on virtual error location

Let $c = [c_1,...c_n] \in \mathrm{GF}(q)^n =\mathrm{GF}(2^8)^n$ be a given data chunk, and $c_i$ be a byte, i.e., an element over $\mathrm{GF}(2^8)$. We assume an $(n, k)$ RS code $\mathcal{C}$ is employed.

In our implementation, $c$ is first simply split into two subvectors: The left-most $k$ bytes

$$
c_l = [c_1,c_2,...c_k] \in \mathrm{GF}(2^8)^k,\\
$$

and the right-most $n-k$ bytes

$$
c_r = [c_{k+1},...c_n] \in \mathrm{GF}(2^8)^{n-k}.
$$

Here we regard $c_r$ as a part containing errors and $c_l$ as error-free. This means **we fix virtual error locations at the right-most $n-k$ symbols of the given chunk**.

## 2. Derivation of a corresponding codeword of $\mathcal{C}$ only from the first $k$ symbols of a chunk $c$

We see that $(n,k)$ RS codes are maximum distance separable (MDS) and hence a codeword can be reconstructed at least its error-free $k$-out-of-$n$ symbols of any combination. Thus, as we fixed virtual error positions above, i.e., $c_r$, we can identify $c_l$ as a codeword (i.e., base) $w \in \mathcal{C}$. In other words, obviously, there exist an isomorphism $\phi$, $\mathrm{GF}(2^8)^k \rightarrow \mathcal{C}$, $\phi: c_r \mapsto w$.

In the deduplication process, we then obtain the codeword $c_w$ uniquely corresponding to the chunk $c$ from $c_l$, that is, we have $w = \phi(c_l)$. Here, we suppose this bijection is expressed by a **systematic generator matrix** $G = \left[ \begin{array}{cc} I & P \end{array} \right] \in \mathrm{GF}(2^8)^{k \times n}$ of $\mathcal{C}$, where $I$ is a $k \times k$ identity matrix. Namely, we have a codeword `w` by the following calculation:

$$
w = c_l G
   = c_l \left[ \begin{array}{cc} I & P \end{array} \right]
   = \left[ \begin{array}{cc} c_l & c_l P \end{array} \right] \in \mathcal{C}.
$$

Then, for the codeword $w$, the deviation, i.e., virtual additive error, $e$ is easily computed in such a way that $c = w + e$ is satisfied:

$$
e = \left[0,\dots,0, c_r - c_l P \right].
$$

This means that the error part, i.e., deviation, is aligned to the right-most $n-k$ symbols of the chunk. Thus the deviation can be expressed as a vector of $n-k$ bytes.

## 3. Representation of base and deviation in deduplicated output and dictionary

In deduplicated data and dictionary in deduplication instances, the base $w$ and deviation $e$ are expressed as $c_l \in \mathrm{GF}(2^8)^k$ and $c_r - c_l P \in \mathrm{GF}(2^8)^{n-k}$, respectively. This is because $c_l$ can be identified as $w \in \mathcal{C}$ as we mentioned. Also $c_r - c_l P$ is identified as $e$ from its special structure due to the systematic generator matrix and fixed positions of virtual errors.

---

## Rationale and more reasonable approach for generic data based on *error-alignment*

Observe that in GD, we can arbitrarily assume *positions of virtual additive errors* in a given chunk to calculate the deviation. In the above method, we simply suppose that the message part $c_l = [c_1,...,c_k]$ of chunk $c$ is error-free. Thus, we can uniquely fix the base, i.e., $w = c_l G \sim c_l$, and the deviation $e = [0,...,0, c_r- c_l P] \sim c_r - c_l P$ as well. Thus we can execute GD by applying this constraint.

However, *since virtual additive errors are fluctuations in given data for its centered base, they would not always be contained at the right-most $n-k$ symbols of an $n$-byte chunk $c$* even if we carefully choose parameters $n$ and $k$ according to the given data type/structure. Thus, in order to reasonably apply an $(n,k)$ RS code for more generic data type/structure in GD, **we should also configure the positions of virtual additive errors as well.**

To this end, we can take an approach of the *"error alignment"* or *"pushing errors aside"* on given chunk by precoding data chunks (by applying `GD.align_error(m: Vec<Vec<u8>>)` method).

The very basic idea of error-alignment is given in the following paper in terms of *reordering high entropy bits*:

> Vestergaard, Rasmus, Daniel E. Lucani, and Qi Zhang. "Generalized deduplication: Lossless compression for large amounts of small IoT data." European Wireless 2019; 25th European Wireless Conference. VDE, 2019.

In our concept, the idea is a bit more generalized by employing *lienar transformation* instead of reordering (permutation). In particular for a specific data type, we first fix a linear transformation $T: \mathrm{GF}(2^8) \rightarrow \mathrm{GF}(2^8)^n$, i.e., being multiplied a nonsingular $n x n$ matrix $T \in \mathrm{GF}(2^8)^{n \times n}$. Note that the simplest $T$ is typically a simple permutation matrix to align error symbols to the last positions, as given in the above paper. We then execute the precoding on a given chunk $c$ as follows.

$$
[x_l, x_r] = cT \in \mathrm{GF}(2^8)^n,
$$

where $x_l \in \mathrm{GF}(2^8)^k$ and $x_r \in \mathrm{GF}(2^8)^{n-k}$. Then, the base $w$ and the deviation $e$ are calculated on $[x_l, x_r]$ instead of $[c_l, c_r]$ by the above approach, as follows:

$$
w = x_l G
   = [x_l, x_l P],
$$

and

$$
e = [0,...,0, x_r - x_l P],
$$

and $x_l$ is recoded as a base and $x_r - x_l P$ is regarded as a deviation in a deduplicated data stream and the GD dictionary.

We should note that *the linear transformation $T$ pushes the virtual errors contained in $c$ of the specific data form to the right-most $n-k$ symbols of a transformed chunk of length $n$.*

The above operations are simply concatenated into the following:

$$
T= \left[\begin{array}{cc} T_l & T_r \end{array} \right],\\
w = c T_l G = [c T_l, c T_l P],\\
e = [0,...,0, c T_r - c T_l P],
$$

where $T_l \in GF(2^8)^{n \times k}$ and $T_r \in GF(2^8)^{n \times {(n-k)}}$.

Since it is known that we need to properly configure virtual error positions to achieve better deduplication performance, code length and error-positions have been considered to be dynamically adjusted by splitting a chunk into subchunks forming a specific pattern of fluctuationss. In contrast, the error-alignment approach simply align errors in data chunk to the last positions, and a data chunk is processed by a single GD instance with single code parameter.

**The most important factor to achieve better deduplication rate in GD is the estimation of fluctuation/virtual-error patterns contained in given data chunks**.
