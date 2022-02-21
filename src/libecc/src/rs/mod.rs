use super::{error::*, math::*, types::*, ByteUnitCode, Code, Decoded, Encoded};

#[derive(Debug, Clone)]
pub struct ReedSolomon {
  pub code_symbol_len: usize,             // n over GF(2^8)
  pub info_symbol_len: usize,             // k over GF(2^8)
  pub deviation_symbol_len: usize,        // deviation length over GF(2^8)
  generator_matrix_parity: Matrix<GF256>, // parity part P of systematic generator matrix G = [I P] as a look-up table for encoding
}

impl ReedSolomon {
  pub fn new(code_symbol_len: usize, info_symbol_len: usize) -> Result<Self> {
    ensure!(
      code_symbol_len > info_symbol_len && code_symbol_len < ORDER && info_symbol_len < ORDER,
      "Invalid params"
    );

    let vandermonde_matrix_src = Matrix::new(&{
      (0..info_symbol_len)
        .map(|row| {
          (0..code_symbol_len)
            .map(|col| GF256(ROOT).pow((row * col) as isize))
            .collect()
        })
        .collect::<Vec<Vec<GF256>>>()
    });
    ensure!(
      vandermonde_matrix_src.is_ok(),
      "Failed to instantiate RS generator matrix"
    );
    let vandermonde_matrix = vandermonde_matrix_src.unwrap();

    let inverse_matrix_src = vandermonde_matrix
      .clone()
      .inverse_left_submatrix(GF256(0), GF256(1));
    ensure!(
      inverse_matrix_src.is_ok(),
      "Failed to make inversion of RS generator matrix"
    );

    // Systematic generator matrix for ease
    let systematic_generator_matrix = inverse_matrix_src.unwrap() * vandermonde_matrix;
    let parity_part_src = systematic_generator_matrix
      .clone()
      .column_submat(info_symbol_len, code_symbol_len);
    ensure!(parity_part_src.is_ok(), "Failed to get parity submatrix");

    Ok(ReedSolomon {
      code_symbol_len,
      info_symbol_len,
      deviation_symbol_len: code_symbol_len - info_symbol_len, // redundancy length
      generator_matrix_parity: parity_part_src.unwrap(),
    })
  }
}

impl ByteUnitCode for ReedSolomon {
  fn code_byte_len(&self) -> usize {
    self.code_symbol_len
  }
  fn info_byte_len(&self) -> usize {
    self.info_symbol_len
  }
}
impl Code for ReedSolomon {
  type Slice = U8SRep;
  type Vector = U8VRep;

  fn decode(&self, data: &Self::Slice) -> Result<Decoded<Self::Vector>> {
    ensure!(data.len() == self.code_symbol_len, "Invalid data length");
    // note that the first info_symbol_len symbols are assumed to be error-free
    // c = uG = u[I P] = [u uP]
    let error_free_message = &data[0..self.info_symbol_len];
    let error_free_dev = vec![0u8; self.deviation_symbol_len];

    let error_free_encoded = if let Ok(v) = self.encode(error_free_message, &error_free_dev) {
      v
    } else {
      bail!("Failed to process data");
    };

    let base = (&error_free_encoded.codeword[0..self.info_symbol_len]).to_vec();

    let errored_deviation = &data[self.info_symbol_len..];
    let deviation = (&error_free_encoded.codeword[self.info_symbol_len..])
      .iter()
      .zip(errored_deviation.iter())
      .map(|(v, w)| (GF256(*v) + GF256(*w)).0)
      .collect();

    Ok(Decoded::<Self::Vector> { base, deviation })
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
    let msg_gf256 = if let Ok(m) = Matrix::new(&vec![message
      .into_iter()
      .map(|x| GF256(*x))
      .collect::<Vec<GF256>>()])
    {
      m
    } else {
      bail!("Something wrong in matrix conversion of message")
    };
    let parity_gf256 = msg_gf256 * self.generator_matrix_parity.clone();
    ensure!(parity_gf256.row_size() == 1, "Failed to encode");

    // Deviation is defined as the difference between error-free codeword and erroneous one at the redundancy part of the codeword.
    let errored_parity_gf256 = parity_gf256.0[0].clone()
      + Vectorized(dev.into_iter().map(|v| GF256(*v)).collect::<Vec<GF256>>());

    let parity: Vec<u8> = parity_gf256.0[0].0.iter().map(|x| x.0).collect();
    let mut codeword = message.to_vec().clone();
    codeword.extend_from_slice(parity.as_slice());

    let errored_parity: Vec<u8> = errored_parity_gf256.0.iter().map(|v| v.0).collect();
    let mut errored = message.to_vec().clone();
    errored.extend_from_slice(errored_parity.as_slice());

    Ok(Encoded::<Self::Vector> { codeword, errored })
  }
}

#[cfg(test)]
mod tests {
  use crate::HexDump;

  use super::*;

  const N: usize = 10;
  const K: usize = 4;

  #[test]
  fn decode_works() {
    let rs = ReedSolomon::new(N, K).unwrap();
    let message = (0u8..K as u8).map(|x| x).collect::<U8VRep>();
    let dev = &[0u8; N - K];
    let encoded = rs.encode(&message, dev).unwrap();
    let decoded = rs.decode(&encoded.errored).unwrap();

    assert_eq!(message, decoded.base);
    assert_eq!(dev.to_vec(), decoded.deviation);
    assert_eq!(message.hexdump().unwrap(), decoded.base.hexdump().unwrap());
    assert_eq!(
      dev.as_slice().hexdump().unwrap(),
      decoded.deviation.hexdump().unwrap()
    );

    let message = (0u8..K as u8).map(|x| x).collect::<U8VRep>();
    let dev = (0u8..(N - K) as u8).rev().map(|x| x).collect::<U8VRep>();
    let encoded = rs.encode(&message, &dev).unwrap();
    let decoded = rs.decode(&encoded.errored).unwrap();

    assert_eq!(message, decoded.base);
    assert_eq!(dev, decoded.deviation);
    assert_eq!(message.hexdump().unwrap(), decoded.base.hexdump().unwrap());
    assert_eq!(dev.hexdump().unwrap(), decoded.deviation.hexdump().unwrap())
  }

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
    let ans_cw_parity = rs
      .generator_matrix_parity
      .0
      .iter()
      .fold(Vectorized(vec![GF256(0); N - K]), |acc, v| acc + v.clone())
      .0
      .iter()
      .map(|gf| gf.0)
      .collect::<U8VRep>();
    let ans_err_parity: U8VRep = ans_cw_parity
      .iter()
      .enumerate()
      .map(|(i, v)| *v ^ dev[i])
      .collect();
    let mut ans_cw = message.to_vec();
    let mut ans_err = message.to_vec();
    ans_cw.extend_from_slice(ans_cw_parity.as_slice());
    ans_err.extend_from_slice(ans_err_parity.as_slice());
    assert_eq!(encoded.codeword, ans_cw);
    assert_eq!(encoded.errored, ans_err);

    let message = (0u8..K as u8).map(|x| x).collect::<U8VRep>();
    let dev = &[0u8; N - K];
    let encoded = rs.encode(&message, dev).unwrap();
    let ans_cw_parity = rs
      .generator_matrix_parity
      .0
      .iter()
      .enumerate()
      .fold(Vectorized(vec![GF256(0); N - K]), |acc, (row_idx, v)| {
        acc + v.clone().mul_scalar(GF256(row_idx as u8))
      })
      .0
      .iter()
      .map(|gf| gf.0)
      .collect::<U8VRep>();
    let mut ans_cw = message.to_vec();
    ans_cw.extend_from_slice(ans_cw_parity.as_slice());
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
      rs.generator_matrix_parity,
      Matrix::new(&vec![
        vec![
          GF256(64),
          GF256(231),
          GF256(229),
          GF256(158),
          GF256(164),
          GF256(178)
        ],
        vec![
          GF256(120),
          GF256(210),
          GF256(191),
          GF256(71),
          GF256(219),
          GF256(188)
        ],
        vec![
          GF256(54),
          GF256(87),
          GF256(7),
          GF256(140),
          GF256(217),
          GF256(213)
        ],
        vec![
          GF256(15),
          GF256(99),
          GF256(92),
          GF256(84),
          GF256(167),
          GF256(218)
        ]
      ])
      .unwrap()
    );
  }
}
