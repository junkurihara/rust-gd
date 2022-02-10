mod field;
mod matrix;
mod vectorized;

use super::{error::*, Code, Decoded, Encoded};
use field::{GF256, ORDER, ROOT};

#[derive(Debug, Clone)]
pub struct ReedSolomon {
  pub code_symbol_len: usize,            // n over GF(2^8)
  pub info_symbol_len: usize,            // k over GF(2^8)
  pub deviation_symbol_len: usize,       // deviation length over GF(2^8)
  pub generator_matrix: Vec<Vec<GF256>>, // generator matrix G as a look-up table for encoding
  pub inverse_matrix: Vec<Vec<GF256>>, // inverse matrix M satisfying MG = [I P], where I is a k x k identity matrix
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
      deviation_symbol_len: code_symbol_len - info_symbol_len, // redundancy length
      generator_matrix,
      inverse_matrix: vec![vec![]], // TODO:
    })
  }
}

impl Code for ReedSolomon {
  type Slice = [u8];
  type Vector = Vec<u8>;

  fn decode(&self, data: &Self::Slice) -> Result<Decoded<Self::Vector>> {
    ensure!(data.len() == self.code_symbol_len, "Invalid data length");

    Ok(Decoded::<Self::Vector> {
      base: vec![],
      deviation: vec![],
    })
  }

  fn encode(&self, message: &Self::Slice, dev: &Self::Slice) -> Result<Encoded<Self::Vector>> {
    ensure!(
      message.len() == self.info_symbol_len,
      "Invalid message length"
    );
    ensure!(
      dev.len() == self.deviation_symbol_len,
      "Invalid deviation length"
    );
    let msg_gf256: Vec<GF256> = message.into_iter().map(|x| GF256(*x)).collect();
    // TODO: Should this be a systematic generator matrix for ease?
    let codeword_gf256 = self.generator_matrix.iter().enumerate().fold(
      vec![GF256(0u8); self.code_symbol_len],
      |acc, (row_idx, base)| {
        acc
          .into_iter()
          .enumerate()
          .map(|(col_idx, acc_elem)| acc_elem + msg_gf256[row_idx].clone() * base[col_idx].clone())
          .collect()
      },
    );
    let codeword = codeword_gf256.iter().map(|x| x.0).collect();
    // Deviation is defined as the difference between error-free codeword and erroneous one at the redundancy part of the codeword.
    let errored: Vec<u8> = codeword_gf256
      .into_iter()
      .enumerate()
      .map(|(idx, x)| {
        if idx < self.info_symbol_len {
          x.0
        } else {
          (x + GF256(dev[idx - self.info_symbol_len])).0
        }
      })
      .collect();

    Ok(Encoded::<Self::Vector> { codeword, errored })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  const N: usize = 10;
  const K: usize = 4;

  #[test]
  fn encode_works() {
    let rs = ReedSolomon::new(N, K).unwrap();
    let message = &[0u8; K];
    let dev = &[0u8; N - K];
    let encoded = rs.encode(message, dev).unwrap();
    assert_eq!(encoded.codeword, vec![0u8; N]);
    assert_eq!(encoded.errored, vec![0u8; N]);

    let message = &[1u8; K];
    let dev = &[1u8; N - K];
    let encoded = rs.encode(message, dev).unwrap();
    let ans_cw = rs.generator_matrix.iter().fold(vec![0u8; N], |acc, v| {
      v.iter().zip(acc.iter()).map(|(x, y)| x.0 ^ y).collect()
    });
    let ans_err: Vec<u8> = ans_cw
      .iter()
      .enumerate()
      .map(|(i, v)| if i < K { *v } else { *v ^ dev[i - K] })
      .collect();
    assert_eq!(encoded.codeword, ans_cw);
    assert_eq!(encoded.errored, ans_err);

    let message = (0u8..K as u8).map(|x| x).collect::<Vec<u8>>();
    let dev = &[0u8; N - K];
    let encoded = rs.encode(&message, dev).unwrap();
    let ans_cw = rs
      .generator_matrix
      .iter()
      .enumerate()
      .fold(vec![0u8; N], |acc, (row_idx, v)| {
        v.iter()
          .zip(acc.iter())
          .map(|(x, y)| (GF256(row_idx as u8) * x.clone()).0 ^ y)
          .collect()
      });
    assert_eq!(encoded.codeword, ans_cw);
    assert_eq!(encoded.errored, ans_cw);
  }

  #[test]
  fn new_works() {
    let rs = ReedSolomon::new(N, K).unwrap();
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
