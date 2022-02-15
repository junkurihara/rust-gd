use crate::types::*;
use bitvec::prelude::*;

pub fn u32_to_u8vec(num: &u32) -> Vec<u8> {
  let mask: u32 = 0xFF;
  let u8vec: Vec<u8> = (0..4)
    .into_iter()
    .rev()
    .map(|idx| {
      let shift_mask = mask << (idx * 8);
      ((num & shift_mask) >> (idx * 8)) as u8
    })
    .collect();
  u8vec
}

pub fn u8vec_to_msb(u8vec: &[u8]) -> BVRep {
  BitVec::from_slice(u8vec)
}

pub fn u32_to_msb(num: &u32) -> BVRep {
  u8vec_to_msb(&u32_to_u8vec(num))
}

pub fn msb_to_u32(bv: &BSRep) -> u32 {
  assert!(bv.len() <= 32);
  let r = bv.iter().rev().enumerate().fold(
    0u32,
    |acc, (idx, b)| if *b { acc + (1 << idx) } else { acc },
  );
  r
}

#[allow(dead_code)]
pub fn get_residue(num: &BSRep, poly: &BSRep) -> BVRep {
  let first_one = poly.first_one().unwrap();
  let deg = poly.len() - first_one - 1;
  let trailed_poly = poly[first_one..].to_bitvec();

  if trailed_poly.len() > num.len() {
    let mut base = bitvec![u8, Msb0; 0; trailed_poly.len() - num.len() - 1];
    base.extend_from_bitslice(num);
    return base;
  }

  let mut aligned_poly = bitvec![u8, Msb0; 0; num.len() - trailed_poly.len()];
  aligned_poly.extend_from_bitslice(&trailed_poly);

  let res = get_residue_bits(num, &aligned_poly, deg);
  res[res.len() - deg..].to_bitvec()
}

#[allow(dead_code)]
fn get_residue_bits(num: &BSRep, poly: &BSRep, deg: usize) -> BVRep {
  assert_eq!(num.len(), poly.len());

  let ord = if let Some(one) = num.first_one() {
    num.len() - one - 1
  } else {
    return num.to_bitvec();
  };

  if ord < deg {
    num.to_bitvec()
  } else {
    let mut shifted = poly.to_bitvec();
    shifted.shift_left(ord - deg);
    get_residue_bits(&(shifted ^ num), poly, deg)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_residue_with_leading_zeros() {
    let bit_num = BVRep::from_element(0b10000);
    let bit_poly = BVRep::from_element(0b1011);
    let res = get_residue(&bit_num, &bit_poly);
    assert_eq!(res, bitvec![u8, Msb0; 1, 1, 0]);

    let bit_num = BVRep::from_element(0b1000_0000);
    let bit_poly = BVRep::from_element(0b10011);
    let res = get_residue(&bit_num, &bit_poly);
    assert_eq!(res, bitvec![u8, Msb0; 1, 0, 1, 1]);

    let bit_num = BVRep::from_vec(vec![0x80u8, 0x00u8]);
    let bit_poly = BVRep::from_vec(vec![0b1, 0x1Du8]);
    let res = get_residue(&bit_num, &bit_poly);
    assert_eq!(res, BVRep::from_element(0x26u8));
  }

  #[test]
  fn test_residue() {
    let bit_poly: BVRep = bitvec![u8, Msb0; 1, 0, 0, 1, 1];
    let ba: BVRep = bitvec![u8, Msb0; 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0];
    assert_eq!(bitvec![u8, Msb0; 0, 1, 0, 1], get_residue(&ba, &bit_poly));

    let bit_poly: BVRep = bitvec![u8, Msb0; 1, 0, 0, 1, 1];
    let ba: BVRep = bitvec![u8, Msb0; 0, 1, 0];
    assert_eq!(bitvec![u8, Msb0; 0, 0, 1, 0], get_residue(&ba, &bit_poly));
  }

  #[test]
  fn test_u32_to_bitvec_msb() {
    let u8vec = u32_to_u8vec(&0xFF10);
    assert_eq!(u8vec, vec![0u8, 0, 0xFF, 0x10]);

    let bv = u8vec_to_msb(&u8vec);
    let exp_bv = bits![
      0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 1, 0, 0, 0,
      0,
    ];
    assert_eq!(bv, exp_bv.to_bitvec());
    let bv = u32_to_msb(&0xFF10);
    assert_eq!(bv, exp_bv.to_bitvec());
  }

  #[test]
  fn test_msb_to_u32() {
    let bv = bitvec![u8, Msb0; 1, 0];
    assert_eq!(2, msb_to_u32(&bv));

    let bv = bitvec![u8, Msb0; 1, 0, 0, 0];
    assert_eq!(8, msb_to_u32(&bv));

    let bv = bitvec![u8, Msb0; 0, 1, 0, 1, 0, 0, 0, 0];
    assert_eq!(80, msb_to_u32(&bv));

    let bv = bitvec![u8, Msb0; 1, 1, 1, 1, 1, 1, 1, 1];
    assert_eq!(255, msb_to_u32(&bv));

    let bv = bitvec![u8, Msb0; 1, 1, 1, 1, 1, 1, 1, 1, 1];
    assert_eq!(511, msb_to_u32(&bv));
  }
}
