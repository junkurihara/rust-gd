use bitvec::prelude::*;
use std::convert::From;

pub enum Separator {
  Deduped,
  AsIs,
}

impl From<bool> for Separator {
  fn from(b: bool) -> Self {
    if b {
      Separator::Deduped
    } else {
      Separator::AsIs
    }
  }
}

impl Separator {
  pub fn bv(&self) -> BitVec {
    match *self {
      Separator::Deduped => bitvec![1],
      Self::AsIs => bitvec![0],
    }
  }
}
