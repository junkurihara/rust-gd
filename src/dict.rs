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
    // println!(
    // "{:?} {:?} {:?} {:?}\n{:?}\n{:?}",
    // kr, k, vr, v, self.dict_base_to_id, self.dict_id_to_base
    // );
    ensure!(kr == k && vr == v, "Failed to remove...Broken dictionary");

    Ok(v)
  }

  pub fn check_inner_integrity(&self) -> Result<()> {
    // check consistency between id_to_base and base_to_id
    ensure!(
      self.dict_base_to_id.len() == self.dict_id_to_base.len(),
      "Different size of dictionary"
    );

    let res: Vec<Result<()>> = self
      .dict_base_to_id
      .iter()
      .map(|(base, id)| {
        if let Some(base_r) = self.dict_id_to_base.get(id) {
          if *base_r == *base {
            Ok(())
          } else {
            Err(anyhow!("a base is inconsistent for id {}", id))
          }
        } else {
          Err(anyhow!("a base is missing in dict_id_to_base"))
        }
      })
      .collect();
    ensure!(res.iter().all(|x| (*x).is_ok()), "Inconsistent dictionary");

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  const WORD_STR: &str = "寿限無(じゅげむ)寿限無(じゅげむ)五劫(ごこう)のすりきれ海砂利(かいじゃり)水魚(すいぎょ)の水行末(すいぎょうまつ) ";
  const MSG_BYTELEN: usize = 3;
  const CODE_BYTELEN: usize = 4;
  const DICT_SIZE: usize = 10;

  #[test]
  fn dict_u8_sync_works() {
    let word_bytes = WORD_STR.to_string().repeat(128).into_bytes();
    let mut basis_dict_1 = BasisDict::<U8VRep>::new(DICT_SIZE);
    let mut basis_dict_2 = BasisDict::<U8VRep>::new(DICT_SIZE);

    let mut byte_ptr: usize = 0;
    while byte_ptr < word_bytes.len() {
      if byte_ptr + MSG_BYTELEN > word_bytes.len() {
        break;
      }

      let buf = &word_bytes[byte_ptr..byte_ptr + MSG_BYTELEN];
      if let Some(bit_id) = basis_dict_1.get_id(&buf.to_vec()) {
        let base_r = basis_dict_2.get_base(&bit_id).unwrap();
        assert_eq!(base_r, buf.to_vec());
      } else {
        let _new_id = basis_dict_1.put_base(&buf.to_vec()).unwrap();
        // simulate existing usecase
        let mut bv_buf = BVRep::from_slice(&buf.to_vec()).clone();
        bv_buf.force_align();
        let u8buf = bv_buf.to_bitvec().as_raw_slice().to_owned();
        let _new_id_r = basis_dict_2.put_base(&u8buf).unwrap();
        assert_eq!(_new_id, _new_id_r);
      }
      assert!(basis_dict_1.check_inner_integrity().is_ok());
      assert!(basis_dict_2.check_inner_integrity().is_ok());

      byte_ptr += CODE_BYTELEN;
    }
  }
}
