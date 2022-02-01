use crate::error::*;
use bitvec::prelude::*;

pub fn hexdump_bitslice<T: BitStore, O: BitOrder>(bs: &BitSlice<T, O>) -> Result<String> {
  let mut res = BitVec::<T, O>::new();
  if bs.len() % 8 > 0 {
    res.extend_from_bitslice(&bitvec![u8, O; 0; 8-bs.len()%8]);
  }
  res.extend_from_bitslice(bs);
  let (pfx, mid, sfx) = unsafe { res.align_to::<u8>() };
  ensure!(
    pfx.len() == 0 && mid.len() % 8 == 0 && sfx.len() == 0,
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
  fn hexdump_test() {
    assert_eq!(
      "0FFF",
      hexdump_bitslice(bitvec![u8,Msb0;1;12].as_bitslice()).unwrap()
    );
    assert_eq!(
      "00AA",
      hexdump_bitslice(bitvec![u8,Msb0;0,1,0,1,0,1,0,1,0].as_bitslice()).unwrap()
    );

    (0u8..0xFF).for_each(|x| {
      let bv = BitVec::<u8, Msb0>::from_element(x);
      assert_eq!(
        format!("{:02X}", x),
        hexdump_bitslice(bv.as_bitslice()).unwrap()
      );
    })
  }
}
