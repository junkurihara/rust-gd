mod error;
mod util;

mod hamming;

use crate::error::*;
use bitvec::prelude::*;
pub use hamming::Hamming;
pub use util::{bitdump_bitslice, hexdump_bitslice};

pub trait Code {
  type Slice: ?Sized;
  type Vector;

  fn encode(&self, info: &Self::Slice, dev: &Self::Slice) -> Result<Encoded<Self::Vector>>;
  fn decode(&self, data: &Self::Slice) -> Result<Decoded<Self::Vector>>;
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
