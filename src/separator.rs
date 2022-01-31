use bitvec::prelude::*;

pub enum Separator {
  Deduped,
  AsIs,
}

impl Separator {
  pub fn bv(&self) -> BitVec {
    match *self {
      Separator::Deduped => bitvec![1],
      Self::AsIs => bitvec![0],
    }
  }
}
