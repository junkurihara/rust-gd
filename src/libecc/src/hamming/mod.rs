mod constant;
mod util;

use super::{error::*, types::*, BitUnitCode, Code, Decoded, Encoded};
use bitvec::prelude::*;
use constant::{ERROR_POS_TO_SYNDROME, SYNDROME_TO_ERROR_POS};
use util::{msb_to_u32, u32_to_msb};

#[derive(Debug, Clone)]
pub struct Hamming {
  pub deg: u32,                        // m
  pub code_bit_len: usize,             // n
  pub info_bit_len: usize,             // k
  pub error_pos_to_syndrome: Vec<u32>, // error position -> syndrome value
  pub syndrome_to_error_pos: Vec<u32>, // syndrome (expressed in usize msb) -> one error bit idx, idx=0 then no error
}

impl Hamming {
  pub fn new(deg: u32) -> Result<Self> {
    let code_len = (2u32.pow(deg) - 1) as usize;
    let info_len = code_len - deg as usize;

    let error_pos_to_syndrome = ERROR_POS_TO_SYNDROME.get(&deg).unwrap().to_vec();
    let syndrome_to_error_pos = SYNDROME_TO_ERROR_POS.get(&deg).unwrap().to_vec();
    Ok(Hamming {
      code_bit_len: code_len,
      info_bit_len: info_len,
      deg,
      error_pos_to_syndrome,
      syndrome_to_error_pos,
    })
  }

  fn calc_syndrome(&self, cw: &BSRep) -> BVRep {
    let syndrome_len = self.code_bit_len - self.info_bit_len;
    cw.iter()
      .enumerate()
      .fold(bitvec![u8, Msb0; 0; syndrome_len], |acc, (pos, b)| {
        if *b {
          let pos_syn = &u32_to_msb(&self.error_pos_to_syndrome[pos + 1])[32 - syndrome_len..];
          acc ^ pos_syn
        } else {
          acc
        }
      })
  }

  fn one_bit_flip_by_syndrome(&self, data: &BSRep, syn: &BSRep) -> BVRep {
    let mut flipped = data.to_bitvec();
    let syn_val = msb_to_u32(syn);
    if syn_val > 0 {
      let error_pos = self.syndrome_to_error_pos[syn_val as usize] as usize - 1;
      let pos_val = flipped[error_pos];
      flipped.set(error_pos, !pos_val);
    }
    flipped
  }
}

impl BitUnitCode for Hamming {
  fn info_bit_len(&self) -> usize {
    self.info_bit_len
  }
  fn code_bit_len(&self) -> usize {
    self.code_bit_len
  }
}
impl Code for Hamming {
  type Slice = BSRep;
  type Vector = BVRep;

  fn decode(&self, data: &Self::Slice) -> Result<Decoded<Self::Vector>> {
    ensure!(data.len() == self.code_bit_len, "Invalid data length");

    let syn = self.calc_syndrome(data);
    let no_error = self.one_bit_flip_by_syndrome(data, &syn);
    let info = (&no_error[0..self.info_bit_len]).to_bitvec();
    ensure!(
      info.len() == self.info_bit_len && syn.len() == self.deg as usize,
      "Invalid calc result"
    );

    Ok(Decoded::<Self::Vector> {
      deviation: syn,
      base: info,
    })
  }

  fn encode(&self, info: &Self::Slice, dev: &Self::Slice) -> Result<Encoded<Self::Vector>> {
    ensure!(
      info.len() == self.info_bit_len && dev.len() == self.deg as usize,
      "Invalid data length"
    );

    let mut cw = info.to_bitvec();
    cw.extend_from_bitslice(&bitvec![u8, Msb0; 0; self.code_bit_len - self.info_bit_len]);
    let parity = self.calc_syndrome(&cw);
    let mut res = info.to_bitvec();
    res.extend_from_bitslice(&parity);
    ensure!(res.len() == self.code_bit_len, "Invalid calc result");
    let flipped = self.one_bit_flip_by_syndrome(&res, dev);
    ensure!(
      flipped.len() == self.code_bit_len,
      "Invalid error calculation"
    );

    Ok(Encoded::<Self::Vector>(flipped))
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
  use constant::POLYNOMIALS;
  use util::{get_residue, u8vec_to_msb};

  use super::*;

  #[test]
  fn test_deg3_bits() {
    let code_len = 7;
    let hamming = Hamming::new(3).unwrap();

    let data: BVRep = bitvec![u8, Msb0; 0; code_len];
    let syndrome = hamming.decode(data.as_bitslice()).unwrap();
    assert_eq!("00", syndrome.base.hexdump().unwrap());
    assert_eq!("00", syndrome.deviation.hexdump().unwrap());
    assert_eq!("0000", syndrome.base.bitdump());
    assert_eq!("000", syndrome.deviation.bitdump());

    let data: BVRep = bitvec![u8, Msb0; 1; code_len];
    let syndrome = hamming.decode(data.as_bitslice()).unwrap();
    assert_eq!("1111", syndrome.base.bitdump());
    assert_eq!("000", syndrome.deviation.bitdump());

    let data: BVRep = bitvec![u8, Msb0; 1,0,1,1,1,1,0];
    let syndrome = hamming.decode(&data).unwrap();
    assert_eq!("1001", syndrome.base.bitdump());
    assert_eq!("110", syndrome.deviation.bitdump());

    let data = bitvec![u8, Msb0; 1,1,0,0,1,1,1];
    let syndrome = hamming.decode(&data).unwrap();
    assert_eq!("0100", syndrome.base.bitdump());
    assert_eq!("101", syndrome.deviation.bitdump());

    let data = bitvec![u8, Msb0; 0,0,0,0,1,0,1];
    let syndrome = hamming.decode(&data).unwrap();
    assert_eq!("1000", syndrome.base.bitdump());
    assert_eq!("101", syndrome.deviation.bitdump());

    let data = bitvec![u8, Msb0; 1,0,0,0];
    let syndrome = bitvec![u8,Msb0; 0,0,0];
    let parity = hamming.encode(&data, &syndrome).unwrap();
    assert_eq!("1000101", parity.0.bitdump());

    let data = bitvec![u8, Msb0; 1,0,1,0];
    let syndrome = bitvec![u8,Msb0; 1,1,0];
    let parity = hamming.encode(&data, &syndrome).unwrap();
    assert_eq!("1000011", parity.0.bitdump());

    let data = bitvec![u8, Msb0; 1,0,0,0];
    let syndrome = bitvec![u8,Msb0; 1,0,1];
    let parity = hamming.encode(&data, &syndrome).unwrap();
    assert_eq!("0000101", parity.0.bitdump());
  }

  #[test]
  fn test_validate_table() {
    for deg in 3..11 {
      let code_len = (2u32.pow(deg) - 1) as usize;

      let error_pos_to_syndrome = *ERROR_POS_TO_SYNDROME.get(&deg).unwrap();
      let syndrome_to_error_pos = *SYNDROME_TO_ERROR_POS.get(&deg).unwrap();

      let poly = u8vec_to_msb(*POLYNOMIALS.get(&deg).unwrap());
      (0usize..code_len + 1).for_each(|error_pos| {
        let mut error = bitvec![u8, Msb0; 0; code_len];
        if error_pos > 0 {
          error.set(error_pos - 1, true);
        };
        let syndrome = get_residue(&error, &poly);
        let syndrome_value = msb_to_u32(&syndrome) as usize;

        assert_eq!(error_pos_to_syndrome[error_pos] as usize, syndrome_value);
        assert_eq!(error_pos, syndrome_to_error_pos[syndrome_value] as usize);

        let syndrome_bit = u32_to_msb(&(syndrome_value as u32));
        let syndrome_val2 = msb_to_u32(&syndrome_bit);
        assert_eq!(syndrome, &syndrome_bit.as_bitslice()[32 - syndrome.len()..]);
        assert_eq!(syndrome_val2, syndrome_value as u32);
      });
    }
  }

  /*
    #[test]
    fn test_deg8_bits() {
      let code_len = 255;
      let hamming = Hamming::new(8).unwrap();

      let data: BVRep = bitvec![u8, Msb0; 0; code_len];
      let syndrome = hamming.decode(&data);
      assert_eq!("0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000", syndrome.bitdump_info());
      assert_eq!("00000000", syndrome.bitdump_syndrome());
      // assert_eq!("000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000", syndrome.bitdump_noerror());
      let data: BVRep = bitvec![u8, Msb0; 1; code_len];
      let syndrome = hamming.decode(&data);
      assert_eq!("1111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111", syndrome.bitdump_info());
      assert_eq!("00000000", syndrome.bitdump_syndrome());
      // assert_eq!("111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111", syndrome.bitdump_noerror());
    }

  */
}
