mod constant;
mod util;

use super::{error::*, Code, Decoded, Encoded};
use bitvec::prelude::*;
use constant::POLYNOMIALS;
use util::{get_residue, msb_to_u32, u8vec_to_msb};

#[derive(Debug, Clone)]
pub struct Hamming {
  pub deg: u32,                            // m
  pub code_len: usize,                     // n
  pub info_len: usize,                     // k
  pub syndrome_tbl: Vec<BitVec<u8, Msb0>>, // error vec -> syndrome
  pub syndrome_tbl_rev: Vec<u32>, // syndrome (expressed in usize msb) -> one error bit idx, idx=0 then no error
}

impl Hamming {
  pub fn new(deg: u32) -> Result<Self, Error> {
    let poly = if let Some(p) = POLYNOMIALS.get(&deg) {
      u8vec_to_msb(*p)
    } else {
      bail!("Unsupported degree");
    };
    let code_len = (2u32.pow(deg) - 1) as usize;
    let info_len = code_len - deg as usize;
    let synd_len = deg as usize;

    let mut syndrome_tbl = vec![bitvec![u8, Msb0; 0; synd_len]; code_len + 1];
    let mut syndrome_tbl_rev: Vec<u32> = (0..code_len + 1).into_iter().map(|_| 0).collect();
    for pos in 0..code_len {
      //let bv = &(u32_to_msb(&(1 << pos)))[32 - code_len as usize..];
      let bv = &{
        let mut base = bitvec![u8, Msb0; 0; code_len];
        base.set(pos, true);
        base
      };
      let res = get_residue(bv, &poly);
      let res_val = msb_to_u32(&res);
      syndrome_tbl[pos + 1] = (&res[res.len() - synd_len..]).to_bitvec();
      syndrome_tbl_rev[res_val as usize] = pos as u32 + 1;
    }

    Ok(Hamming {
      code_len,
      info_len,
      deg,
      syndrome_tbl,
      syndrome_tbl_rev,
    })
  }

  fn calc_syndrome(&self, cw: &BitSlice<u8, Msb0>) -> BitVec<u8, Msb0> {
    cw.iter().enumerate().fold(
      bitvec![u8, Msb0; 0; self.code_len - self.info_len],
      |acc, (pos, b)| {
        if *b {
          let pos_syn = &self.syndrome_tbl[pos + 1];
          acc ^ pos_syn
        } else {
          acc
        }
      },
    )
  }

  fn one_bit_flip_by_syndrome(
    &self,
    data: &BitSlice<u8, Msb0>,
    syn: &BitSlice<u8, Msb0>,
  ) -> BitVec<u8, Msb0> {
    let mut flipped = data.to_bitvec();
    let syn_val = msb_to_u32(syn);
    if syn_val > 0 {
      let error_pos = self.syndrome_tbl_rev[syn_val as usize] as usize;
      let pos_val = flipped[error_pos - 1];
      flipped.set(error_pos - 1, !pos_val);
    }
    flipped
  }
}

impl Code for Hamming {
  type Slice = BitSlice<u8, Msb0>;
  type Vector = BitVec<u8, Msb0>;

  fn decode(&self, data: &Self::Slice) -> Result<Decoded<Self::Vector>> {
    ensure!(data.len() == self.code_len, "Invalid data length");

    let syn = self.calc_syndrome(data);
    let no_error = self.one_bit_flip_by_syndrome(data, &syn);
    let info = (&no_error[0..self.info_len]).to_bitvec();
    ensure!(
      info.len() == self.info_len && syn.len() == self.deg as usize,
      "Invalid calc result"
    );

    Ok(Decoded::<Self::Vector> {
      // no_error: noerror,
      syndrome: syn,
      info,
    })
  }

  fn encode(&self, info: &Self::Slice, dev: &Self::Slice) -> Result<Encoded<Self::Vector>> {
    ensure!(
      info.len() == self.info_len && dev.len() == self.deg as usize,
      "Invalid data length"
    );

    let mut cw = info.to_bitvec();
    cw.extend_from_bitslice(&bitvec![u8, Msb0; 0; self.code_len - self.info_len]);
    let parity = self.calc_syndrome(&cw);
    let mut res = info.to_bitvec();
    res.extend_from_bitslice(&parity);
    ensure!(res.len() == self.code_len, "Invalid calc result");
    let flipped = self.one_bit_flip_by_syndrome(&res, dev);
    ensure!(flipped.len() == self.code_len, "Invalid error calculation");

    Ok(Encoded::<Self::Vector> {
      errored: flipped,
      codeword: res,
    })
  }
}

// Generator matrix: MSB <-> LSB
// [ 1000 101
//   0100 111
//   0010 110
//   0001 011 ]
// Parity check matrix:
// [ 1110 100
//   0111 010
//   1101 001 ]
#[cfg(test)]
mod tests {
  use crate::{BitDump, HexDump};

  use super::*;
  // use constant::test_vectors::*;

  #[test]
  fn test_deg3_bits() {
    let code_len = 7;
    let hamming = Hamming::new(3).unwrap();

    let data: BitVec<u8, Msb0> = bitvec![u8, Msb0; 0; code_len];
    let syndrome = hamming.decode(data.as_bitslice()).unwrap();

    assert_eq!("00", syndrome.info.hexdump().unwrap());
    assert_eq!("00", syndrome.syndrome.hexdump().unwrap());
    assert_eq!("0000", syndrome.info.bitdump());
    assert_eq!("000", syndrome.syndrome.bitdump());

    let data: BitVec<u8, Msb0> = bitvec![u8, Msb0; 1; code_len];
    let syndrome = hamming.decode(data.as_bitslice()).unwrap();
    assert_eq!("1111", syndrome.info.bitdump());
    assert_eq!("000", syndrome.syndrome.bitdump());

    let data: BitVec<u8, Msb0> = bitvec![u8, Msb0; 1,0,1,1,1,1,0];
    let syndrome = hamming.decode(&data).unwrap();
    assert_eq!("1001", syndrome.info.bitdump());
    assert_eq!("110", syndrome.syndrome.bitdump());

    let data = bitvec![u8, Msb0; 1,1,0,0,1,1,1];
    let syndrome = hamming.decode(&data).unwrap();
    assert_eq!("0100", syndrome.info.bitdump());
    assert_eq!("101", syndrome.syndrome.bitdump());

    let data = bitvec![u8, Msb0; 0,0,0,0,1,0,1];
    let syndrome = hamming.decode(&data).unwrap();
    assert_eq!("1000", syndrome.info.bitdump());
    assert_eq!("101", syndrome.syndrome.bitdump());

    let data = bitvec![u8, Msb0; 1,0,0,0];
    let syndrome = bitvec![u8,Msb0; 0,0,0];
    let parity = hamming.encode(&data, &syndrome).unwrap();
    assert_eq!("1000101", parity.errored.bitdump());
    assert_eq!("1000101", parity.codeword.bitdump());

    let data = bitvec![u8, Msb0; 1,0,1,0];
    let syndrome = bitvec![u8,Msb0; 1,1,0];
    let parity = hamming.encode(&data, &syndrome).unwrap();
    assert_eq!("1000011", parity.errored.bitdump());
    assert_eq!("1010011", parity.codeword.bitdump());

    let data = bitvec![u8, Msb0; 1,0,0,0];
    let syndrome = bitvec![u8,Msb0; 1,0,1];
    let parity = hamming.encode(&data, &syndrome).unwrap();
    assert_eq!("1000101", parity.codeword.bitdump());
    assert_eq!("0000101", parity.errored.bitdump());
  }

  /*
    #[test]
    fn test_deg8_bits() {
      let code_len = 255;
      let hamming = Hamming::new(8).unwrap();

      let data: BitVec<u8, Msb0> = bitvec![u8, Msb0; 0; code_len];
      let syndrome = hamming.decode(&data);
      assert_eq!("0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000", syndrome.bitdump_info());
      assert_eq!("00000000", syndrome.bitdump_syndrome());
      // assert_eq!("000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000", syndrome.bitdump_noerror());
      let data: BitVec<u8, Msb0> = bitvec![u8, Msb0; 1; code_len];
      let syndrome = hamming.decode(&data);
      assert_eq!("1111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111", syndrome.bitdump_info());
      assert_eq!("00000000", syndrome.bitdump_syndrome());
      // assert_eq!("111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111", syndrome.bitdump_noerror());
    }

  */
}
