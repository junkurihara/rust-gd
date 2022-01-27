mod error;

use crate::error::*;
use bitvec::prelude::*;
use lib_hamming::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct GenDedup {
  code: Hamming,
  dict_size: usize,
  id_bitlen: u32,
  dict_idx_to_base: Vec<BitVec<u8, Msb0>>,
  dict_base_to_idx: HashMap<BitVec<u8, Msb0>, usize>,
}

impl GenDedup {
  pub fn new(deg: u32) -> Result<Self, Error> {
    if let Ok(code) = Hamming::new(deg) {
      let dict_size = code.info_len;
      let id_bitlen = 0usize.leading_zeros() - code.info_len.leading_zeros();
      Ok(GenDedup {
        code,
        dict_size, // TODO: This must be know to the receiver
        id_bitlen,
        dict_base_to_idx: HashMap::new(),
        dict_idx_to_base: Vec::new(),
      })
    } else {
      bail!("Failed to instantiate associated Hamming code");
    }
  }

  pub fn dedup(&mut self, buf: &[u8]) -> Result<BitVec<u8, Msb0>, Error> {
    match self.code.get_syndrome(buf) {
      Ok(synd) => {
        // println!("{:?}", buf);
        // println!("{}", synd.dump_info());
        // println!("{}", synd.dump_syndrome());
        // println!("{}", synd.dump_noerror());
        let mut res = BitVec::<u8, Msb0>::new();
        let block_num = if synd.info.len() % self.code.info_len > 0 {
          synd.info.len() / self.code.info_len + 1
        } else {
          synd.info.len() / self.code.info_len
        };
        for block_idx in 0..block_num {
          let offset_info = block_idx * self.code.info_len;
          let offset_synd = block_idx * self.code.deg as usize;
          let bs_info = &synd.info[offset_info..offset_info + self.code.info_len];
          let bs_synd = &synd.syndrome[offset_synd..offset_synd + self.code.deg as usize];

          // handle dict maxsize
          if self.dict_idx_to_base.len() >= self.dict_size {
            self.flush_dict();
            println!("flush dictionary");
          }

          if self.dict_base_to_idx.contains_key(bs_info) {
            let id = self.dict_base_to_idx.get(bs_info).unwrap();
            println!("found base: id {:4X}", id); //: {}", bs_info);
            let bs_id: BitVec<usize, Msb0> = BitVec::from_element(*id);
            res.extend_from_bitslice(&bs_id[bs_id.len() - self.id_bitlen as usize..]);
          } else {
            self
              .dict_base_to_idx
              .insert(bs_info.to_bitvec(), self.dict_idx_to_base.len());
            self.dict_idx_to_base.push(bs_info.to_bitvec());
            res.extend_from_bitslice(bs_info);
          }
          res.extend_from_bitslice(bs_synd);
        }

        // println!("{}", hexdump(res.as_raw_slice()));
        Ok(res)
      }
      Err(e) => Err(e),
    }
  }

  fn flush_dict(&mut self) {
    self.dict_base_to_idx = HashMap::new();
    self.dict_idx_to_base = Vec::new();
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_deg3() {
    let gd = GenDedup::new(3).unwrap();
  }
}
