use crate::error::*;
use crate::types::*;
use bitvec::prelude::*;
use hashlink::LinkedHashMap;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct BasisDict<T>
where
  T: std::cmp::Eq + std::hash::Hash + std::clone::Clone + std::fmt::Debug,
{
  dict_size: usize,
  id_bitlen: usize,
  dict_id_to_base: HashMap<usize, T>,
  dict_base_to_id: LinkedHashMap<T, usize>,
  // TODO: Add a table to manage LRU
}

impl<T> BasisDict<T>
where
  T: std::cmp::Eq + std::hash::Hash + std::clone::Clone + std::fmt::Debug,
{
  pub fn new(dict_size: usize) -> Self {
    let id_bitlen = (0usize.leading_zeros() - dict_size.leading_zeros()) as usize;
    BasisDict {
      dict_size,
      id_bitlen,
      dict_id_to_base: HashMap::new(),
      dict_base_to_id: LinkedHashMap::new(),
    }
  }

  pub fn id_bitlen(&self) -> usize {
    self.id_bitlen
  }

  // call only in dedup
  pub fn get_id(&mut self, base: &T) -> Option<IdRep> {
    if let Some(id) = self.dict_base_to_id.get(base) {
      let res = self.usize_id_to_bitvec_id(id);
      self.dict_base_to_id.to_back(base); // update internal linked list
      Some(res)
    } else {
      None
    }
  }

  // call in dedup when id was not found in get_id
  // call in dup when base is given
  pub fn put_base(&mut self, base: &T) -> Result<IdRep> {
    // LRU
    let new_id = if self.dict_base_to_id.len() < self.dict_size {
      self.dict_base_to_id.len()
    } else {
      self.remove_lru_entry_get_freed_id()?
    };
    self.dict_base_to_id.insert(base.to_owned(), new_id);
    self.dict_id_to_base.insert(new_id, base.to_owned());
    // println!("> newid = {}", new_id);
    let res = self.usize_id_to_bitvec_id(&new_id);

    Ok(res)
  }

  // call only in dup when id is given
  pub fn get_base(&mut self, bit_id: &IdSRep) -> Result<T> {
    // https://github.com/bitvecto-rs/bitvec/issues/119
    let mut id = 0usize;
    for (mut dst, src) in id.view_bits_mut::<Lsb0>()[..bit_id.len()]
      .iter_mut()
      .zip(bit_id.iter().rev())
    {
      dst.set(*src);
    }

    let base = self
      .dict_id_to_base
      .get(&id)
      .ok_or(())
      .map_err(|_| anyhow!("Invalid dictionary"))?;
    self.dict_base_to_id.to_back(base); // update internal linked list

    Ok(base.to_owned())
  }

  fn usize_id_to_bitvec_id(&self, id: &usize) -> IdRep {
    let bs_id: BitVec<usize, Msb0> = BitVec::from_element(id.to_owned());
    let mut res = BVRep::new();
    res.extend_from_bitslice(&bs_id[bs_id.len() - self.id_bitlen as usize..]);
    res
  }

  fn remove_lru_entry_get_freed_id(&mut self) -> Result<usize> {
    // 1. pop LRU entry from linked hash map (base-to-id) and get dropped id
    let (k, v) = self
      .dict_base_to_id
      .pop_front()
      .ok_or(())
      .map_err(|_| anyhow!("Invalid dictionary"))?;

    // 2. drop entry from hash map (id-to-base)
    let (vr, kr) = self
      .dict_id_to_base
      .remove_entry(&v)
      .ok_or(())
      .map_err(|_| anyhow!("Invalid dictionary"))?;

    // 3. return the id.
    ensure!(kr == k && vr == v, "Failed to remove...Broken dictionary");

    Ok(v)
  }
}
