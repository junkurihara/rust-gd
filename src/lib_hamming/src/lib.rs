use bitvec::prelude::*;
mod constant;
mod error;
mod util;

use constant::POLYNOMIALS;
use error::*;
use util::{get_residue, msb_to_u32, u8vec_to_msb};

pub struct Syndrome {
  pub no_error: BitVec<u8, Msb0>,
  pub info: BitVec<u8, Msb0>,
  pub syndrome: BitVec<u8, Msb0>,
  pub padding_bits_msb: usize,
}

#[derive(Debug, Clone)]
pub struct Hamming {
  deg: u32,                            // m
  code_len: usize,                     // n
  info_len: usize,                     // k
  syndrome_tbl: Vec<BitVec<u8, Msb0>>, // error vec -> syndrome
  syndrome_tbl_rev: Vec<u32>, // syndrome (expressed in usize msb) -> one error bit idx, idx=0 then no error
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
        base.set(code_len - pos - 1, true);
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

  pub fn get_syndrome(&self, data: &[u8]) -> Result<Syndrome, Error> {
    let bv: BitVec<u8, Msb0> = if let Ok(b) = BitVec::<u8, Msb0>::try_from_slice(data) {
      b
    } else {
      bail!("Failed to read data in bit field");
    };
    let redundant = bv.len() % self.code_len as usize;
    let block_num = if redundant > 0 {
      (bv.len() - redundant) / self.code_len as usize + 1
    } else {
      bv.len() / self.code_len as usize
    };

    let mut bitvec_syndrome = BitVec::<u8, Msb0>::new();
    let mut bitvec_noerror = BitVec::<u8, Msb0>::new();
    let mut bitvec_info = BitVec::<u8, Msb0>::new();
    let mut padding_bits_msb = 0;
    for bidx in 0..block_num {
      let sliced = if bidx < block_num - 1 {
        &bv[bidx * self.code_len..(bidx + 1) * self.code_len]
      } else {
        // no padding here. will pad later in no_error from MSB
        &bv[bidx * self.code_len..]
      };

      let syn = sliced.iter().rev().enumerate().fold(
        bitvec![u8, Msb0; 0; self.code_len - self.info_len],
        |acc, (pos, b)| {
          if *b {
            let pos_syn = &self.syndrome_tbl[pos + 1];
            acc ^ pos_syn
          } else {
            acc
          }
        },
      );

      let mut noerror = if sliced.len() < self.code_len {
        // padded from MSB
        padding_bits_msb = self.code_len - sliced.len();
        let mut base = bitvec![u8, Msb0; 0; padding_bits_msb];
        base.extend_from_bitslice(sliced);
        base
      } else {
        sliced.to_bitvec()
      };
      let syn_val = msb_to_u32(&syn);
      if syn_val > 0 {
        let error_pos = self.code_len - self.syndrome_tbl_rev[syn_val as usize] as usize;
        let pos_val = noerror[error_pos];
        noerror.set(error_pos, !pos_val);
      }
      // println!("{}", noerror);

      bitvec_syndrome.extend_from_bitslice(&syn);
      bitvec_noerror.extend_from_bitslice(&noerror);
      bitvec_info.extend_from_bitslice(&noerror[self.code_len - self.info_len..])
    }

    Ok(Syndrome {
      no_error: bitvec_noerror,
      syndrome: bitvec_syndrome,
      info: bitvec_info,
      padding_bits_msb,
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use constant::test_vectors::*;

  fn hex(bytes: &[u8]) -> String {
    bytes
      .iter()
      .fold("".to_owned(), |s, b| format!("{}{:02X}", s, b))
  }

  #[test]
  fn test_deg3() {
    // Generator matrix: MSB <-> LSB
    // [ 101 1000
    //   111 0100
    //   110 0010
    //   011 0001 ]
    // Parity check matrix:
    // [ 100 1110
    //   010 0111
    //   001 1101 ]
    let hamming = Hamming::new(3).unwrap();

    let data: Vec<u8> = (0..255).into_iter().collect();
    let syndrome = hamming.get_syndrome(&data).unwrap();

    assert_eq!(hex(syndrome.syndrome.as_raw_slice()), TEST_VEC_3_SYN);
    assert_eq!(hex(syndrome.no_error.as_raw_slice()), TEST_VEC_3_NOERR);
    assert_eq!(hex(syndrome.info.as_raw_slice()), TEST_VEC_3_INFO);
  }

  #[test]
  fn test_deg4() {
    let hamming = Hamming::new(4).unwrap();

    let data: Vec<u8> = (0..255).into_iter().collect();
    let syndrome = hamming.get_syndrome(&data).unwrap();

    assert_eq!(hex(syndrome.syndrome.as_raw_slice()), TEST_VEC_4_SYN);
    assert_eq!(hex(syndrome.no_error.as_raw_slice()), TEST_VEC_4_NOERR);
    assert_eq!(hex(syndrome.info.as_raw_slice()), TEST_VEC_4_INFO);
  }

  #[test]
  fn test_deg8() {
    let hamming = Hamming::new(8).unwrap();

    let data: Vec<u8> = (0..255).into_iter().collect();
    let syndrome = hamming.get_syndrome(&data).unwrap();

    assert_eq!(hex(syndrome.syndrome.as_raw_slice()), TEST_VEC_8_SYN);
    assert_eq!(hex(syndrome.no_error.as_raw_slice()), TEST_VEC_8_NOERR);
    assert_eq!(hex(syndrome.info.as_raw_slice()), TEST_VEC_8_INFO);
  }
}
