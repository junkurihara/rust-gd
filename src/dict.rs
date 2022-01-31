use crate::error::*;
use crate::separator::*;
use bitvec::prelude::*;
use hashlink::LinkedHashMap;
use std::collections::HashMap;

// Dictionary of ID-Base and Base-ID with eviction method of LRU
#[derive(Debug, Clone)]
pub struct BaseDict {
  dict_size: usize,
  id_bitlen: usize,
  dict_id_to_base: HashMap<usize, BitVec<u8, Msb0>>,
  dict_base_to_id: LinkedHashMap<BitVec<u8, Msb0>, usize>,
  // TODO: Add a table to manage LRU
}

impl BaseDict {
  pub fn new(dict_size: usize) -> Self {
    let id_bitlen = (0usize.leading_zeros() - dict_size.leading_zeros()) as usize;
    BaseDict {
      dict_size,
      id_bitlen,
      dict_id_to_base: HashMap::new(),
      dict_base_to_id: LinkedHashMap::new(),
    }
  }

  pub fn get_id_bitlen(&self) -> usize {
    self.id_bitlen
  }

  pub fn get_id_or_base(
    &mut self,
    base: &BitSlice<u8, Msb0>,
  ) -> Result<(Separator, BitVec<u8, Msb0>), Error> {
    let mut res = BitVec::<u8, Msb0>::new();
    if let Some(id) = self.dict_base_to_id.get(base) {
      // println!("found base: id {:4X}", id); //: {}", bs_base);
      let bs_id: BitVec<usize, Msb0> = BitVec::from_element(id.to_owned());
      res.extend_from_bitslice(&bs_id[bs_id.len() - self.id_bitlen as usize..]);
      self.dict_base_to_id.to_back(base); // update internal linked list
      Ok((Separator::Deduped, res))
    } else {
      // LRU
      let new_id = if self.dict_base_to_id.len() < self.dict_size {
        self.dict_base_to_id.len()
      } else if let Ok(v) = self.remove_lru_entry_get_freed_id() {
        v
      } else {
        bail!("Invalid dictionary");
      };
      self.dict_base_to_id.insert(base.to_bitvec(), new_id);
      self.dict_id_to_base.insert(new_id, base.to_bitvec());
      // println!("> newid = {}", new_id);
      res.extend_from_bitslice(base);
      Ok((Separator::AsIs, res))
    }
  }

  pub fn get_base(
    &mut self,
    base_or_id: &BitSlice<u8, Msb0>,
    sep: Separator,
  ) -> Result<BitVec<u8, Msb0>, Error> {
    match sep {
      Separator::Deduped => {
        // base_or_id is id
        // https://github.com/bitvecto-rs/bitvec/issues/119
        let mut id = 0usize;
        for (mut dst, src) in id.view_bits_mut::<Lsb0>()[..base_or_id.len()]
          .iter_mut()
          .zip(base_or_id.iter().rev())
        {
          dst.set(*src);
        }

        // println!("id: > {}", id);
        let base = if let Some(b) = self.dict_id_to_base.get(&id) {
          b
        } else {
          bail!("Invalid dictionary");
        };
        self.dict_base_to_id.to_back(base); // update internal linked list

        Ok(base.to_bitvec())
        // Ok((Separator::Deduped, res))
      }
      Separator::AsIs => {
        // base_or_id is base
        let new_id = if self.dict_base_to_id.len() < self.dict_size {
          self.dict_base_to_id.len()
        } else if let Ok(v) = self.remove_lru_entry_get_freed_id() {
          v
        } else {
          bail!("Invalid dictionary");
        };
        self.dict_base_to_id.insert(base_or_id.to_bitvec(), new_id);
        self.dict_id_to_base.insert(new_id, base_or_id.to_bitvec());
        // println!("> newid = {}", new_id);
        Ok(base_or_id.to_bitvec())
      }
    }
  }

  fn remove_lru_entry_get_freed_id(&mut self) -> Result<usize, Error> {
    // 1. pop LRU entry from linked hash map (base-to-id) and get dropped id
    let (k, v) = if let Some(p) = self.dict_base_to_id.pop_front() {
      p
    } else {
      bail!("Invalid dictionary");
    };
    // 2. drop entry from hash map (id-to-base)
    let (vr, kr) = if let Some(p) = self.dict_id_to_base.remove_entry(&v) {
      p
    } else {
      bail!("Invalid dictionary");
    };
    // 3. return the id.
    if kr != k || vr != v {
      bail!("Failed to remove...Broken dictionary");
    }
    // println!("> drop {}", v);
    Ok(v)
  }
}
