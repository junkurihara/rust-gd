mod error;
pub mod types;
mod util;

mod hamming;
mod rs;

use bitvec::prelude::*;
use error::*;
pub use hamming::Hamming;
pub use rs::ReedSolomon;
use types::*;
pub use util::{bitdump_bitslice, hexdump_bitslice, hexdump_slice};

pub trait Code {
  type Slice: ?Sized;
  type Vector;

  fn encode(&self, info: &Self::Slice, dev: &Self::Slice) -> Result<Encoded<Self::Vector>>;
  fn decode(&self, data: &Self::Slice) -> Result<Decoded<Self::Vector>>;
}
pub trait BitUnitCode: Code<Slice = BSRep, Vector = BVRep> {
  fn code_bit_len(&self) -> usize;
  fn info_bit_len(&self) -> usize;
}
pub trait ByteUnitCode: Code<Slice = U8SRep, Vector = U8VRep> {
  fn code_byte_len(&self) -> usize;
  fn info_byte_len(&self) -> usize;
}

#[derive(Debug, Clone)]
pub struct Decoded<T> {
  pub base: T,
  pub deviation: T,
}

#[derive(Debug, Clone)]
pub struct Encoded<T> {
  pub codeword: T,
  pub errored: T,
}

pub trait BitDump {
  fn bitdump(&self) -> String;
}
impl<T: BitStore, O: BitOrder> BitDump for BitSlice<T, O> {
  fn bitdump(&self) -> String {
    bitdump_bitslice(self)
  }
}
impl<T: BitStore, O: BitOrder> BitDump for BitVec<T, O> {
  fn bitdump(&self) -> String {
    bitdump_bitslice(self)
  }
}

pub trait HexDump {
  fn hexdump(&self) -> Result<String>;
}
impl<T: BitStore, O: BitOrder> HexDump for BitSlice<T, O> {
  fn hexdump(&self) -> Result<String> {
    hexdump_bitslice(self)
  }
}
impl<T: BitStore, O: BitOrder> HexDump for BitVec<T, O> {
  fn hexdump(&self) -> Result<String> {
    hexdump_bitslice(self.as_bitslice())
  }
}
impl HexDump for &U8SRep {
  fn hexdump(&self) -> Result<String> {
    hexdump_slice(self)
  }
}
impl HexDump for U8VRep {
  fn hexdump(&self) -> Result<String> {
    hexdump_slice(self.as_slice())
  }
}
