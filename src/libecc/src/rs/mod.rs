mod field;

use super::{error::*, Code, Decoded, Encoded};
use field::{GF256, ORDER, ROOT};

#[derive(Debug, Clone)]
pub struct ReedSolomon {
  pub code_symbol_len: usize,            // n over GF(2^8)
  pub info_symbol_len: usize,            // k over GF(2^8)
  pub generator_matrix: Vec<Vec<GF256>>, // generator matrix
}

impl ReedSolomon {
  pub fn new(code_symbol_len: usize, info_symbol_len: usize) -> Result<Self> {
    ensure!(
      code_symbol_len > info_symbol_len && code_symbol_len < ORDER && info_symbol_len < ORDER,
      "Invalid params"
    );
    let generator_matrix: Vec<Vec<GF256>> = (0..info_symbol_len)
      .map(|row| {
        (0..code_symbol_len)
          .map(|col| GF256(ROOT).pow((row * col) as isize))
          .collect()
      })
      .collect();
    Ok(ReedSolomon {
      code_symbol_len,
      info_symbol_len,
      generator_matrix,
    })
  }
}

impl Code for ReedSolomon {
  type Slice = [u8];
  type Vector = Vec<u8>;

  fn decode(&self, data: &Self::Slice) -> Result<Decoded<Self::Vector>> {
    Ok(Decoded::<Self::Vector> {
      base: vec![],
      deviation: vec![],
    })
  }

  fn encode(&self, info: &Self::Slice, dev: &Self::Slice) -> Result<Encoded<Self::Vector>> {
    Ok(Encoded::<Self::Vector> {
      codeword: vec![],
      errored: vec![],
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn new_works() {
    let rs = ReedSolomon::new(10, 4).unwrap();
    // [
    //  [GF256(1), GF256(1), GF256(1), GF256(1), GF256(1), GF256(1), GF256(1), GF256(1), GF256(1), GF256(1)],
    //  [GF256(1), GF256(2), GF256(4), GF256(8), GF256(16), GF256(32), GF256(64), GF256(128), GF256(29), GF256(58)],
    //  [GF256(1), GF256(4), GF256(16), GF256(64), GF256(29), GF256(116), GF256(205), GF256(19), GF256(76), GF256(45)],
    //  [GF256(1), GF256(8), GF256(64), GF256(58), GF256(205), GF256(38), GF256(45), GF256(117), GF256(143), GF256(12)]
    // ]
    assert_eq!(
      rs.generator_matrix,
      vec![
        vec![
          GF256(1),
          GF256(1),
          GF256(1),
          GF256(1),
          GF256(1),
          GF256(1),
          GF256(1),
          GF256(1),
          GF256(1),
          GF256(1)
        ],
        vec![
          GF256(1),
          GF256(2),
          GF256(4),
          GF256(8),
          GF256(16),
          GF256(32),
          GF256(64),
          GF256(128),
          GF256(29),
          GF256(58)
        ],
        vec![
          GF256(1),
          GF256(4),
          GF256(16),
          GF256(64),
          GF256(29),
          GF256(116),
          GF256(205),
          GF256(19),
          GF256(76),
          GF256(45)
        ],
        vec![
          GF256(1),
          GF256(8),
          GF256(64),
          GF256(58),
          GF256(205),
          GF256(38),
          GF256(45),
          GF256(117),
          GF256(143),
          GF256(12)
        ]
      ]
    );
  }
}
