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
            // separator deduped
            res.extend_from_bitslice(bits![1]);
            res.extend_from_bitslice(&bs_id[bs_id.len() - self.id_bitlen as usize..]);
          } else {
            self
              .dict_base_to_idx
              .insert(bs_info.to_bitvec(), self.dict_idx_to_base.len());
            self.dict_idx_to_base.push(bs_info.to_bitvec());
            // separator as-is
            res.extend_from_bitslice(bits![0]);
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

  pub fn dup(&mut self, deduped: &BitSlice<u8, Msb0>) -> Result<(), Error> {
    // println!("{}", hexdump(deduped.to_bitvec().as_raw_slice()));
    let mut ptr = 0usize;
    let mut info_concat = BitVec::<u8, Msb0>::new();
    loop {
      if ptr == deduped.len() {
        break;
      }
      if self.dict_idx_to_base.len() >= self.dict_size {
        self.flush_dict();
        println!("flush dictionary");
      }

      let separator = deduped[ptr];
      ptr += 1;
      let bs_info = if separator {
        // deduped
        println!("deduped");
        let bs_id: BitVec<u8, Msb0> =
          BitVec::from_bitslice(&deduped[ptr..ptr + self.id_bitlen as usize]);
        let id = bs_id.as_raw_slice()[0] as usize;
        ptr += self.id_bitlen as usize;
        self.dict_idx_to_base.get(id).unwrap()
      } else {
        // as-is
        println!("asis");
        let bs_info = &deduped[ptr..ptr + self.code.info_len];
        self
          .dict_base_to_idx
          .insert(bs_info.to_bitvec(), self.dict_idx_to_base.len());
        self.dict_idx_to_base.push(bs_info.to_bitvec());
        ptr += self.code.info_len as usize;
        bs_info
      };

      let mut bs_codeword = bitvec![u8, Msb0; 0; self.code.deg as usize];
      bs_codeword.extend_from_bitslice(bs_info);
      info_concat.extend_from_bitslice(&bs_codeword);

      ptr += self.code.deg as usize;
    }
    let syndrome = self.code.get_syndrome(info_concat.as_raw_slice());
    println!("{:?}", syndrome);
    // TODO: hamming libをまとめて処理する形をやめないと色々厳しそう。符号語単位で処理するように変更する。

    Ok(())
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
