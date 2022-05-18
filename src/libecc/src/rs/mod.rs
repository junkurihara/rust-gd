use super::{error::*, math::*, types::*, ByteUnitCode, Code, Decoded, Encoded};
use futures::{
  future::join_all,
  stream::{self, StreamExt},
};
use tokio::task::{spawn_blocking, JoinError};

#[derive(Debug, Clone)]
pub struct ReedSolomon {
  pub code_symbol_len: usize,             // n over GF(2^8)
  pub info_symbol_len: usize,             // k over GF(2^8)
  pub deviation_symbol_len: usize,        // deviation length over GF(2^8)
  generator_matrix_parity: Matrix<GF256>, // parity part P of systematic generator matrix G = [I P] as a look-up table for encoding
  precoding: Option<Matrix<GF256>>,       // precoding matrix for error_alignment
  postcoding: Option<Matrix<GF256>>,      // postcoding matrix for error_alignment
}

impl ReedSolomon {
  pub async fn new(code_symbol_len: usize, info_symbol_len: usize) -> Result<Self> {
    ensure!(
      code_symbol_len > info_symbol_len && code_symbol_len < ORDER && info_symbol_len < ORDER,
      "Invalid params"
    );

    let res: Vec<_> = join_all(
      stream::iter(0..info_symbol_len)
        .map(|row| {
          spawn_blocking(move || {
            (0..code_symbol_len)
              .map(|col| GF256(ROOT).pow((row * col) as isize))
              .collect::<Vec<GF256>>()
          })
        })
        .collect::<Vec<_>>()
        .await,
    )
    .await;
    let vandermonde_matrix = Matrix::new(
      &res
        .into_iter()
        .collect::<Result<Vec<Vec<GF256>>, JoinError>>()?,
    )?;

    let inverse_matrix = vandermonde_matrix
      .clone()
      .inverse_left_submatrix(GF256(0), GF256(1))?;

    // Systematic generator matrix for ease
    let systematic_generator_matrix = inverse_matrix * vandermonde_matrix;
    let parity_part = systematic_generator_matrix
      .clone()
      .column_submat(info_symbol_len, code_symbol_len)?;

    Ok(ReedSolomon {
      code_symbol_len,
      info_symbol_len,
      deviation_symbol_len: code_symbol_len - info_symbol_len, // redundancy length
      generator_matrix_parity: parity_part,
      precoding: None,
      postcoding: None,
    })
  }

  fn msg_encode_gf256_within(
    &self,
    message: &Vectorized<GF256>,
    dev: &mut Vectorized<GF256>,
  ) -> Result<()> {
    let parity_gf256 = self
      .generator_matrix_parity
      .clone()
      .mul_on_vec_from_right(message); //Matrix(vec![message.to_owned()]) * self.generator_matrix_parity.clone();

    // Deviation is defined as the difference between error-free codeword and erroneous one at the redundancy part of the codeword.
    dev.add_within(parity_gf256);

    Ok(())
  }
}

impl ByteUnitCode for ReedSolomon {
  fn code_byte_len(&self) -> usize {
    self.code_symbol_len
  }
  fn info_byte_len(&self) -> usize {
    self.info_symbol_len
  }
  fn set_precoding(&mut self, pre: &[U8VRep]) -> Result<()> {
    let mat = Matrix::of_gf256_from_u8(pre);
    ensure!(mat.is_ok(), "Failed to set matrix");
    self.precoding = Some(mat.unwrap());

    let mat = self.precoding.as_ref().unwrap();
    ensure!(mat.is_square(), "Matrix for error alignment must be square");
    let inv = mat
      .inverse_left_submatrix(GF256(0), GF256(1))
      .map_err(|e| anyhow!("Singular matrix: {}", e))?;
    self.postcoding = Some(inv);
    // assert!((mat.clone() * inv.clone()).is_identity_matrix(GF256(0), GF256(1)));
    Ok(())
  }
}
impl Code for ReedSolomon {
  type Slice = U8SRep;
  type Vector = U8VRep;

  fn decode(&self, data: &Self::Slice) -> Result<Decoded<Self::Vector>> {
    ensure!(data.len() == self.code_symbol_len, "Invalid data length");
    let mut precoded = Vectorized::of_gf256_from_u8(data);
    if self.precoding.is_some() {
      precoded = self
        .precoding
        .as_ref()
        .unwrap()
        .mul_on_vec_from_right(&precoded);
    }

    let (message_part, mut parity_part) = (
      precoded.subvec(0, self.info_symbol_len),
      precoded.subvec(self.info_symbol_len, self.code_symbol_len),
    );

    self.msg_encode_gf256_within(&message_part, &mut parity_part)?;

    Ok(Decoded::<Self::Vector> {
      base: message_part.to_u8_vec(),
      deviation: parity_part.to_u8_vec(),
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
    let (msg_gf256, mut dev_gf256) = (
      Vectorized::of_gf256_from_u8(message),
      Vectorized::of_gf256_from_u8(dev),
    );
    self.msg_encode_gf256_within(&msg_gf256, &mut dev_gf256)?;

    let mut cw = msg_gf256.clone();
    cw.extend_from_slice(&dev_gf256.0);

    let postcoded = if self.postcoding.is_some() {
      // TODO: More efficient precoding scheme
      self.postcoding.as_ref().unwrap().mul_on_vec_from_right(&cw)
    } else {
      cw
    };

    Ok(Encoded::<Self::Vector>(postcoded.to_u8_vec()))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::HexDump;

  const N: usize = 10;
  const K: usize = 4;

  #[tokio::test]
  async fn decode_works() {
    let rs = ReedSolomon::new(N, K).await.unwrap();
    let message = (0u8..K as u8).map(|x| x).collect::<U8VRep>();
    let dev = &[0u8; N - K];
    let encoded = rs.encode(&message, dev).unwrap();
    let decoded = rs.decode(&encoded.0).unwrap();

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
    let decoded = rs.decode(&encoded.0).unwrap();

    assert_eq!(message, decoded.base);
    assert_eq!(dev, decoded.deviation);
    assert_eq!(message.hexdump().unwrap(), decoded.base.hexdump().unwrap());
    assert_eq!(dev.hexdump().unwrap(), decoded.deviation.hexdump().unwrap())
  }

  #[tokio::test]
  async fn encode_works() {
    let rs = ReedSolomon::new(N, K).await.unwrap();
    let message = &[0u8; K];
    let dev = &[0u8; N - K];
    let encoded = rs.encode(message, dev).unwrap();
    assert_eq!(encoded.0, vec![0u8; N]);

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
    assert_eq!(encoded.0, ans_err);

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
    assert_eq!(encoded.0, ans_cw);
  }

  #[tokio::test]
  async fn new_works() {
    let rs = ReedSolomon::new(N, K).await.unwrap();
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
