use crate::{error::*, types::*};
use bitvec::prelude::*;

pub fn hexdump_slice(slice: &U8SRep) -> Result<String> {
  let v = slice
    .iter()
    .fold("".to_owned(), |s, b| format!("{}{:02X}", s, b));
  Ok(v)
}

pub fn hexdump_bitslice<T: BitStore, O: BitOrder>(bs: &BitSlice<T, O>) -> Result<String> {
  let mut res = BitVec::<T, O>::new();
  if bs.len() % 8 > 0 {
    res.extend_from_bitslice(&bitvec![u8, O; 0; 8-bs.len()%8]);
  }
  res.extend_from_bitslice(bs);
  let (pfx, mid, sfx) = unsafe { res.align_to::<u8>() };
  ensure!(
    pfx.is_empty() && mid.len() % 8 == 0 && sfx.is_empty(),
    "Invalid bitslice"
  );

  let v = mid
    .to_bitvec()
    .as_raw_slice()
    .iter()
    .fold("".to_owned(), |s, b| format!("{}{:02X}", s, b));
  Ok(v)
}

pub fn bitdump_bitslice<T, O>(bits: &BitSlice<T, O>) -> String
where
  T: BitStore,
  O: BitOrder,
{
  bits.iter().fold("".to_owned(), |s, b| {
    format!("{}{}", s, if *b { 1 } else { 0 })
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn hexdump_slice_test() {
    let sliced = &{ (0usize..256).map(|x| x as u8).collect::<U8VRep>() };
    let ans = "000102030405060708090A0B0C0D0E0F101112131415161718191A1B1C1D1E1F202122232425262728292A2B2C2D2E2F303132333435363738393A3B3C3D3E3F404142434445464748494A4B4C4D4E4F505152535455565758595A5B5C5D5E5F606162636465666768696A6B6C6D6E6F707172737475767778797A7B7C7D7E7F808182838485868788898A8B8C8D8E8F909192939495969798999A9B9C9D9E9FA0A1A2A3A4A5A6A7A8A9AAABACADAEAFB0B1B2B3B4B5B6B7B8B9BABBBCBDBEBFC0C1C2C3C4C5C6C7C8C9CACBCCCDCECFD0D1D2D3D4D5D6D7D8D9DADBDCDDDEDFE0E1E2E3E4E5E6E7E8E9EAEBECEDEEEFF0F1F2F3F4F5F6F7F8F9FAFBFCFDFEFF";
    assert_eq!(ans, hexdump_slice(sliced).unwrap());
  }

  #[test]
  fn hexdump_bitslice_test() {
    assert_eq!(
      "0FFF",
      hexdump_bitslice(bitvec![u8,Msb0;1;12].as_bitslice()).unwrap()
    );
    assert_eq!(
      "00AA",
      hexdump_bitslice(bitvec![u8,Msb0;0,1,0,1,0,1,0,1,0].as_bitslice()).unwrap()
    );

    (0u8..0xFF).for_each(|x| {
      let bv = BVRep::from_element(x);
      assert_eq!(
        format!("{:02X}", x),
        hexdump_bitslice(bv.as_bitslice()).unwrap()
      );
    })
  }
}
