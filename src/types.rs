use bitvec::prelude::*;

pub type IdRep = BitVec<u8, Msb0>;
pub type IdSRep = BitSlice<u8, Msb0>;
pub type BVRep = BitVec<u8, Msb0>;
pub type BSRep = BitSlice<u8, Msb0>;
pub type U8VRep = Vec<u8>;
pub type U8SRep = [u8];
