use bitvec::prelude::*;
mod constant;
mod util;

use super::{
  error::*,
  util::{bitdump, hexdump},
};
use constant::POLYNOMIALS;
use util::{get_residue, msb_to_u32, u8vec_to_msb};

#[derive(Debug, Clone)]
pub struct DecodedWord {
  pub info: BitVec<u8, Msb0>,
  pub syndrome: BitVec<u8, Msb0>,
}

impl DecodedWord {
  pub fn dump_info(&self) -> String {
    hexdump(self.info.as_raw_slice())
  }
  pub fn dump_syndrome(&self) -> String {
    hexdump(self.syndrome.as_raw_slice())
  }
  pub fn bitdump_info(&self) -> String {
    bitdump(self.info.as_bitslice())
  }
  pub fn bitdump_syndrome(&self) -> String {
    bitdump(self.syndrome.as_bitslice())
  }
}

#[derive(Debug, Clone)]
pub struct EncodedWord {
  pub erroneous: BitVec<u8, Msb0>,
  pub no_error: BitVec<u8, Msb0>,
}

impl EncodedWord {
  pub fn dump_errorneous(&self) -> String {
    hexdump(self.erroneous.as_raw_slice())
  }
  pub fn dump_no_error(&self) -> String {
    hexdump(self.no_error.as_raw_slice())
  }
  pub fn bitdump_errorneous(&self) -> String {
    bitdump(self.erroneous.as_bitslice())
  }
  pub fn bitdump_no_error(&self) -> String {
    bitdump(self.no_error.as_bitslice())
  }
}

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

  pub fn decode(&self, data: &BitSlice<u8, Msb0>) -> DecodedWord {
    assert_eq!(data.len(), self.code_len);

    let syn = self.calc_syndrome(data);
    let no_error = self.one_bit_flip_by_syndrome(data, &syn);
    let info = (&no_error[0..self.info_len]).to_bitvec();
    assert_eq!(info.len(), self.info_len);
    assert_eq!(syn.len(), self.deg as usize);

    DecodedWord {
      // no_error: noerror,
      syndrome: syn,
      info,
    }
  }

  // TODO: result型にした方が良い
  pub fn encode(&self, info: &BitSlice<u8, Msb0>, syndrome: &BitSlice<u8, Msb0>) -> EncodedWord {
    assert_eq!(info.len(), self.info_len);
    assert_eq!(syndrome.len(), self.deg as usize);

    let mut cw = info.to_bitvec();
    cw.extend_from_bitslice(&bitvec![u8, Msb0; 0; self.code_len - self.info_len]);
    let parity = self.calc_syndrome(&cw);
    let mut res = info.to_bitvec();
    res.extend_from_bitslice(&parity);
    assert_eq!(res.len(), self.code_len);
    let flipped = self.one_bit_flip_by_syndrome(&res, syndrome);
    assert_eq!(flipped.len(), self.code_len);

    EncodedWord {
      erroneous: flipped,
      no_error: res,
    }
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
  use super::*;
  // use constant::test_vectors::*;

  #[test]
  fn test_deg3_bits() {
    let code_len = 7;
    let hamming = Hamming::new(3).unwrap();

    let data: BitVec<u8, Msb0> = bitvec![u8, Msb0; 0; code_len];
    let syndrome = hamming.decode(&data);
    assert_eq!("0000", syndrome.bitdump_info());
    assert_eq!("000", syndrome.bitdump_syndrome());

    let data: BitVec<u8, Msb0> = bitvec![u8, Msb0; 1; code_len];
    let syndrome = hamming.decode(&data);
    assert_eq!("1111", syndrome.bitdump_info());
    assert_eq!("000", syndrome.bitdump_syndrome());

    let data: BitVec<u8, Msb0> = bitvec![u8, Msb0; 1,0,1,1,1,1,0];
    let syndrome = hamming.decode(&data);
    assert_eq!("1001", syndrome.bitdump_info());
    assert_eq!("110", syndrome.bitdump_syndrome());

    let data = bitvec![u8, Msb0; 1,1,0,0,1,1,1];
    let syndrome = hamming.decode(&data);
    assert_eq!("0100", syndrome.bitdump_info());
    assert_eq!("101", syndrome.bitdump_syndrome());

    let data = bitvec![u8, Msb0; 0,0,0,0,1,0,1];
    let syndrome = hamming.decode(&data);
    assert_eq!("1000", syndrome.bitdump_info());
    assert_eq!("101", syndrome.bitdump_syndrome());

    let data = bitvec![u8, Msb0; 1,0,0,0];
    let syndrome = bitvec![u8,Msb0; 0,0,0];
    let parity = hamming.encode(&data, &syndrome);
    assert_eq!("1000101", parity.bitdump_errorneous());
    assert_eq!("1000101", parity.bitdump_no_error());

    let data = bitvec![u8, Msb0; 1,0,1,0];
    let syndrome = bitvec![u8,Msb0; 1,1,0];
    let parity = hamming.encode(&data, &syndrome);
    assert_eq!("1000011", parity.bitdump_errorneous());
    assert_eq!("1010011", parity.bitdump_no_error());

    let data = bitvec![u8, Msb0; 1,0,0,0];
    let syndrome = bitvec![u8,Msb0; 1,0,1];
    let parity = hamming.encode(&data, &syndrome);
    assert_eq!("1000101", parity.bitdump_no_error());
    assert_eq!("0000101", parity.bitdump_errorneous());
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
