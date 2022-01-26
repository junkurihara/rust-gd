// use std::collections::HashMap;

use bitvec::prelude::*;
mod error;
mod util;

use error::*;
use phf::phf_map;
use util::{get_residue, msb_to_u32, u32_to_msb, u8vec_to_msb};

static POLYS: phf::Map<u32, &'static [u8]> = phf_map! {
  // deg => primitive polynomial
  3u32 => &[0xB],
  4u32 => &[0x13],
  8u32 => &[0x1D, 0b1],
  // 3u32 => BitVec::<u8, Lsb0>::from_vec(vec![0xBu8]), // (7, 4)  : x^3 + x + 1
  // 4u32 => BitVec::<u8, Lsb0>::from_vec(vec![0x13]), // (15, 11): x^4 + x + 1
  // 4u32 => 0x3, // (15, 11): x^4 + x + 1
};

#[derive(Debug, Clone)]
pub struct Hamming {
  deg: u32,                            // m
  code_len: usize,                     // n
  info_len: usize,                     // k
  syndrome_tbl: Vec<BitVec<u8, Msb0>>, // error vec -> syndrome
  syndrome_tbl_rev: Vec<u32>, // syndrome (expressed in usize msb) -> one error bit idx, idx=0 then no error
}

pub struct Syndrome {
  pub no_error: BitVec<u8, Msb0>,
  pub info: BitVec<u8, Msb0>,
  pub syndrome: BitVec<u8, Msb0>,
  pub padding_bits_msb: usize,
}

impl Hamming {
  pub fn new(deg: u32) -> Result<Self, Error> {
    let poly = if let Some(p) = POLYS.get(&deg) {
      u8vec_to_msb(*p)
    } else {
      bail!("Unsupported degree");
    };
    let code_len = (2u32.pow(deg) - 1) as usize;
    let info_len = code_len - deg as usize;
    let synd_len = code_len - info_len;

    let mut syndrome_tbl = vec![bitvec![u8, Msb0; 0; synd_len]; code_len + 1];
    let mut syndrome_tbl_rev: Vec<u32> = (0..code_len + 1).into_iter().map(|_| 0).collect();
    for pos in 0..code_len {
      let bv = &mut (u32_to_msb(&(1 << pos)))[32 - code_len as usize..];
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
            pos_syn
              .iter()
              .zip(&acc)
              .map(|(i, j)| (*i && !*j) || (!*i && *j))
              .collect()
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
      bitvec_info.extend_from_bitslice(&noerror[self.code_len-self.info_len..])
    }

    Ok(Syndrome {
      no_error: bitvec_noerror,
      syndrome: bitvec_syndrome,
      info: bitvec_info,
      padding_bits_msb
    })
  }

  /*
  pub fn encode(&self, info: &[u8]) -> Result<(), Error> {
    let bitrep = if let Ok(bv) = BitVec::<_, Lsb0>::try_from_slice(info) {
      bv
    } else {
      bail!("Failed to encode to bitvect");
    };

    let coded = BitVec::<u8, Lsb0>::new();

    let chunk_num = if info.len() % self.info_len as usize == 0 {
      bitrep.len() / self.info_len as usize
    } else {
      bitrep.len() / self.info_len as usize + 1
    };

    // adjust info vector size
    for i in 0..chunk_num {
      let to_be_coded = if i != chunk_num - 1 {
        bitrep[i * self.info_len as usize..(i + 1) * self.info_len as usize].to_bitvec()
      } else {
        let mut res = bitrep[i * self.info_len as usize..].to_bitvec();
        if info.len() % self.info_len as usize != 0 {
          for _ in 0..self.info_len as usize - bitrep.len() % self.info_len as usize {
            res.push(false);
          }
        }
        res
      };
      // TODO: calc parity here
      println!("{:?}", to_be_coded);
    }
    // println!("{:?}", coded);

    Ok(())
  }
   */
}

#[test]
fn test_7_4() {
  // Generator matrix: MSB <-> LSB
  // [ 101 1000
  //   111 0100
  //   110 0010
  //   011 0001 ]
  // Parity check matrix:
  // [ 100 1110
  //   010 0111
  //   001 1101 ]

  let lib = Hamming::new(3);
  assert!(lib.is_ok());
  let hamming = lib.unwrap();
  // println!("{:?}", hamming);
  let data: Vec<u8> = (0..10).into_iter().collect();
  let bitvec_syndrome_res = hamming.get_syndrome(&data);
  assert!(bitvec_syndrome_res.is_ok());
  let syndrome = bitvec_syndrome_res.unwrap();
  assert_eq!(
    syndrome.syndrome,
    bitvec![0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 1, 0, 0, 0, 1, 1, 1, 0, 0, 1, 1, 1, 1, 1, 0, 0, 1, 0, 0, 1]
  );
  assert_eq!(
    syndrome.no_error,
    bitvec![
      0, 0, 0, 0, 0, 0, 0,
      0, 0, 0, 0, 0, 0, 0,
      0, 0, 0, 0, 0, 0, 0,
      0, 0, 0, 0, 0, 0, 0,
      1, 0, 1, 1, 0, 0, 0,
      0, 0, 0, 0, 0, 0, 0,
      0, 0, 0, 1, 0, 1, 1,
      0, 0, 1, 0, 1, 1, 0,
      0, 0, 0, 1, 0, 1, 1,
      1, 1, 0, 0, 0, 1, 0,
      0, 0, 0, 0, 0, 0, 0,
      0, 0, 0, 0, 0, 0, 0,
    ]
  );
  assert_eq!(
    syndrome.info,
    bitvec![
      0, 0, 0, 0,
      0, 0, 0, 0,
      0, 0, 0, 0,
      0, 0, 0, 0,
      1, 0, 0, 0,
      0, 0, 0, 0,
      1, 0, 1, 1,
      0, 1, 1, 0,
      1, 0, 1, 1,
      0, 0, 1, 0,
      0, 0, 0, 0,
      0, 0, 0, 0,
    ]
  )
  // lib.encode(&[0u8, 1, 2, 3]); // 32 / 4 = 8
}

#[test]
fn test_15_11() {
  let lib = Hamming::new(4);
  assert!(lib.is_ok());
  let hamming = lib.unwrap();
  // println!("{:?}", hamming);
  let data: Vec<u8> = (0..30).into_iter().collect();
  let bitvec_syndrome_res = hamming.get_syndrome(&data);
  assert!(bitvec_syndrome_res.is_ok());
  let syndrome = bitvec_syndrome_res.unwrap();
  assert_eq!(syndrome.no_error, bitvec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 1, 1, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 1, 1, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 0, 1, 0, 1, 1, 0, 0, 1, 0, 1, 0, 1, 1, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 1, 1, 0, 1, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 1, 1, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 0, 1, 0, 1, 1, 1, 0, 0, 0, 1, 0, 1, 0, 0, 0, 0, 0, 1, 0, 1, 0, 1, 0, 0, 1, 1, 0, 1, 1, 0, 0, 0, 0, 1, 0, 1, 1, 1, 1, 0, 0, 1, 1, 0, 0, 0, 1, 0, 0, 1, 1, 0, 0, 1, 0, 0, 0, 1, 1, 0, 1, 0, 0, 0, 0, 0, 1, 0, 1, 1, 0, 0, 0, 1, 1, 1, 0, 0, 0, 0, 0, 1, 0, 1, 0, 1]);
  // lib.encode(&[0u8, 1, 2, 3, 4]); // 40 / 11 = 4
}
