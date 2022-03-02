// Imported from c0dearm/sharks https://github.com/c0dearm/sharks/blob/master/src/field.rs
//
// Basic operations overrided for the Galois Field 256 (2**8)
// Uses pre-calculated tables for 0x11d primitive polynomial (x**8 + x**4 + x**3 + x**2 + 1)

use core::{
  iter::{Product, Sum},
  ops::{Add, Div, Mul, Sub},
};

pub const ORDER: usize = 256;
pub const ROOT: u8 = 0x02;

// LOG_TABLE[i] = log_{alpha} i, the degree of power over alpha
// LOG_TABLE[0] is virtually 0 for simplicity
const LOG_TABLE: [u8; 256] = [
  0x00, 0x00, 0x01, 0x19, 0x02, 0x32, 0x1a, 0xc6, 0x03, 0xdf, 0x33, 0xee, 0x1b, 0x68, 0xc7, 0x4b,
  0x04, 0x64, 0xe0, 0x0e, 0x34, 0x8d, 0xef, 0x81, 0x1c, 0xc1, 0x69, 0xf8, 0xc8, 0x08, 0x4c, 0x71,
  0x05, 0x8a, 0x65, 0x2f, 0xe1, 0x24, 0x0f, 0x21, 0x35, 0x93, 0x8e, 0xda, 0xf0, 0x12, 0x82, 0x45,
  0x1d, 0xb5, 0xc2, 0x7d, 0x6a, 0x27, 0xf9, 0xb9, 0xc9, 0x9a, 0x09, 0x78, 0x4d, 0xe4, 0x72, 0xa6,
  0x06, 0xbf, 0x8b, 0x62, 0x66, 0xdd, 0x30, 0xfd, 0xe2, 0x98, 0x25, 0xb3, 0x10, 0x91, 0x22, 0x88,
  0x36, 0xd0, 0x94, 0xce, 0x8f, 0x96, 0xdb, 0xbd, 0xf1, 0xd2, 0x13, 0x5c, 0x83, 0x38, 0x46, 0x40,
  0x1e, 0x42, 0xb6, 0xa3, 0xc3, 0x48, 0x7e, 0x6e, 0x6b, 0x3a, 0x28, 0x54, 0xfa, 0x85, 0xba, 0x3d,
  0xca, 0x5e, 0x9b, 0x9f, 0x0a, 0x15, 0x79, 0x2b, 0x4e, 0xd4, 0xe5, 0xac, 0x73, 0xf3, 0xa7, 0x57,
  0x07, 0x70, 0xc0, 0xf7, 0x8c, 0x80, 0x63, 0x0d, 0x67, 0x4a, 0xde, 0xed, 0x31, 0xc5, 0xfe, 0x18,
  0xe3, 0xa5, 0x99, 0x77, 0x26, 0xb8, 0xb4, 0x7c, 0x11, 0x44, 0x92, 0xd9, 0x23, 0x20, 0x89, 0x2e,
  0x37, 0x3f, 0xd1, 0x5b, 0x95, 0xbc, 0xcf, 0xcd, 0x90, 0x87, 0x97, 0xb2, 0xdc, 0xfc, 0xbe, 0x61,
  0xf2, 0x56, 0xd3, 0xab, 0x14, 0x2a, 0x5d, 0x9e, 0x84, 0x3c, 0x39, 0x53, 0x47, 0x6d, 0x41, 0xa2,
  0x1f, 0x2d, 0x43, 0xd8, 0xb7, 0x7b, 0xa4, 0x76, 0xc4, 0x17, 0x49, 0xec, 0x7f, 0x0c, 0x6f, 0xf6,
  0x6c, 0xa1, 0x3b, 0x52, 0x29, 0x9d, 0x55, 0xaa, 0xfb, 0x60, 0x86, 0xb1, 0xbb, 0xcc, 0x3e, 0x5a,
  0xcb, 0x59, 0x5f, 0xb0, 0x9c, 0xa9, 0xa0, 0x51, 0x0b, 0xf5, 0x16, 0xeb, 0x7a, 0x75, 0x2c, 0xd7,
  0x4f, 0xae, 0xd5, 0xe9, 0xe6, 0xe7, 0xad, 0xe8, 0x74, 0xd6, 0xf4, 0xea, 0xa8, 0x50, 0x58, 0xaf,
];

// EXP_TABLE[i] = alpha^i, the i-th power of alpha
const EXP_TABLE: [u8; 255] = [
  0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0x1d, 0x3a, 0x74, 0xe8, 0xcd, 0x87, 0x13, 0x26,
  0x4c, 0x98, 0x2d, 0x5a, 0xb4, 0x75, 0xea, 0xc9, 0x8f, 0x03, 0x06, 0x0c, 0x18, 0x30, 0x60, 0xc0,
  0x9d, 0x27, 0x4e, 0x9c, 0x25, 0x4a, 0x94, 0x35, 0x6a, 0xd4, 0xb5, 0x77, 0xee, 0xc1, 0x9f, 0x23,
  0x46, 0x8c, 0x05, 0x0a, 0x14, 0x28, 0x50, 0xa0, 0x5d, 0xba, 0x69, 0xd2, 0xb9, 0x6f, 0xde, 0xa1,
  0x5f, 0xbe, 0x61, 0xc2, 0x99, 0x2f, 0x5e, 0xbc, 0x65, 0xca, 0x89, 0x0f, 0x1e, 0x3c, 0x78, 0xf0,
  0xfd, 0xe7, 0xd3, 0xbb, 0x6b, 0xd6, 0xb1, 0x7f, 0xfe, 0xe1, 0xdf, 0xa3, 0x5b, 0xb6, 0x71, 0xe2,
  0xd9, 0xaf, 0x43, 0x86, 0x11, 0x22, 0x44, 0x88, 0x0d, 0x1a, 0x34, 0x68, 0xd0, 0xbd, 0x67, 0xce,
  0x81, 0x1f, 0x3e, 0x7c, 0xf8, 0xed, 0xc7, 0x93, 0x3b, 0x76, 0xec, 0xc5, 0x97, 0x33, 0x66, 0xcc,
  0x85, 0x17, 0x2e, 0x5c, 0xb8, 0x6d, 0xda, 0xa9, 0x4f, 0x9e, 0x21, 0x42, 0x84, 0x15, 0x2a, 0x54,
  0xa8, 0x4d, 0x9a, 0x29, 0x52, 0xa4, 0x55, 0xaa, 0x49, 0x92, 0x39, 0x72, 0xe4, 0xd5, 0xb7, 0x73,
  0xe6, 0xd1, 0xbf, 0x63, 0xc6, 0x91, 0x3f, 0x7e, 0xfc, 0xe5, 0xd7, 0xb3, 0x7b, 0xf6, 0xf1, 0xff,
  0xe3, 0xdb, 0xab, 0x4b, 0x96, 0x31, 0x62, 0xc4, 0x95, 0x37, 0x6e, 0xdc, 0xa5, 0x57, 0xae, 0x41,
  0x82, 0x19, 0x32, 0x64, 0xc8, 0x8d, 0x07, 0x0e, 0x1c, 0x38, 0x70, 0xe0, 0xdd, 0xa7, 0x53, 0xa6,
  0x51, 0xa2, 0x59, 0xb2, 0x79, 0xf2, 0xf9, 0xef, 0xc3, 0x9b, 0x2b, 0x56, 0xac, 0x45, 0x8a, 0x09,
  0x12, 0x24, 0x48, 0x90, 0x3d, 0x7a, 0xf4, 0xf5, 0xf7, 0xf3, 0xfb, 0xeb, 0xcb, 0x8b, 0x0b, 0x16,
  0x2c, 0x58, 0xb0, 0x7d, 0xfa, 0xe9, 0xcf, 0x83, 0x1b, 0x36, 0x6c, 0xd8, 0xad, 0x47, 0x8e,
];

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct GF256(pub u8);

impl Add for GF256 {
  type Output = GF256;

  fn add(self, other: Self) -> Self::Output {
    Self(self.0 ^ other.0)
  }
}

impl Sub for GF256 {
  type Output = Self;

  fn sub(self, other: Self) -> Self::Output {
    Self(self.0 ^ other.0)
  }
}

impl Mul for GF256 {
  type Output = Self;

  fn mul(self, other: Self) -> Self::Output {
    if self.0 == 0 || other.0 == 0 {
      Self(0)
    } else {
      let log_x = LOG_TABLE[self.0 as usize] as usize;
      let log_y = LOG_TABLE[other.0 as usize] as usize;
      Self(EXP_TABLE[(log_x + log_y) % (ORDER - 1)])
    }
  }
}

impl Div for GF256 {
  type Output = Self;

  fn div(self, other: Self) -> Self::Output {
    if self.0 == 0 {
      Self(0)
    } else {
      let log_x = LOG_TABLE[self.0 as usize] as usize;
      let log_y = LOG_TABLE[other.0 as usize] as usize;

      Self(EXP_TABLE[((ORDER - 1) + log_x - log_y) % (ORDER - 1)])
    }
  }
}

impl Sum for GF256 {
  fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
    iter.fold(Self(0), |acc, x| acc + x)
  }
}

impl Product for GF256 {
  fn product<I: Iterator<Item = Self>>(iter: I) -> Self {
    iter.fold(Self(1), |acc, x| acc * x)
  }
}

impl GF256 {
  pub fn pow(self, exp: isize) -> Self {
    if self.0 == 0 {
      Self(0)
    } else {
      let log_x = LOG_TABLE[self.0 as usize] as usize;
      if exp > 0 {
        Self(EXP_TABLE[(log_x * (exp as usize % (ORDER - 1))) % (ORDER - 1)])
      } else {
        let exp_abs = exp.abs() as usize;
        let p = (ORDER - 1) - exp_abs % (ORDER - 1);
        Self(EXP_TABLE[(p * log_x) % (ORDER - 1)])
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::{EXP_TABLE, GF256, LOG_TABLE, ORDER};
  // use alloc::vec;

  #[test]
  fn add_works() {
    let answers: [u8; 256] = [
      1, 2, 5, 17, 18, 18, 90, 70, 30, 229, 71, 6, 214, 239, 212, 109, 72, 252, 205, 84, 128, 248,
      5, 72, 147, 194, 111, 244, 208, 56, 44, 177, 152, 173, 43, 179, 196, 110, 155, 20, 95, 71,
      59, 173, 30, 211, 29, 102, 91, 57, 199, 119, 126, 15, 169, 25, 148, 32, 96, 170, 244, 139,
      172, 7, 89, 1, 234, 160, 255, 242, 110, 65, 135, 82, 172, 188, 14, 173, 90, 120, 203, 55, 71,
      117, 228, 64, 106, 194, 15, 51, 204, 255, 216, 142, 55, 162, 199, 237, 245, 37, 210, 106, 58,
      230, 102, 32, 28, 60, 42, 56, 221, 243, 75, 65, 165, 227, 242, 248, 190, 184, 117, 162, 9,
      105, 228, 192, 193, 155, 130, 103, 238, 171, 52, 237, 185, 164, 40, 212, 255, 175, 181, 208,
      212, 76, 75, 232, 3, 94, 116, 28, 225, 214, 88, 214, 171, 171, 199, 245, 62, 93, 209, 238,
      110, 56, 83, 45, 240, 179, 108, 98, 64, 1, 167, 10, 79, 158, 17, 141, 120, 224, 130, 27, 63,
      90, 17, 11, 87, 143, 226, 58, 239, 227, 157, 52, 113, 188, 127, 246, 163, 120, 216, 47, 57,
      12, 162, 171, 60, 80, 61, 3, 98, 224, 80, 111, 172, 69, 56, 251, 173, 231, 23, 137, 180, 83,
      217, 125, 23, 32, 161, 211, 84, 164, 252, 6, 237, 0, 177, 254, 39, 193, 99, 246, 101, 148,
      28, 14, 98, 107, 111, 224, 152, 50, 5, 23, 214, 174,
    ];

    // log i + alpha^i for i = 1..255, where alpha^255 = 1
    // alpha_i for i=0
    for (i, a) in answers.iter().enumerate() {
      assert_eq!(
        (GF256(LOG_TABLE[i]) + GF256(EXP_TABLE[i % (ORDER - 1)])).0,
        *a
      );
    }
  }

  #[test]
  fn sub_works() {
    add_works();
  }

  #[test]
  fn mul_works() {
    let answers: [u8; 256] = [
      0, 0, 4, 200, 32, 14, 206, 179, 39, 134, 169, 160, 32, 59, 184, 50, 45, 121, 69, 43, 102, 43,
      139, 169, 18, 94, 107, 84, 18, 157, 159, 51, 211, 1, 52, 13, 51, 128, 31, 219, 240, 230, 212,
      219, 197, 19, 11, 135, 93, 163, 237, 53, 91, 177, 135, 124, 240, 224, 6, 158, 167, 155, 155,
      38, 223, 144, 70, 54, 50, 45, 134, 170, 126, 223, 103, 207, 253, 176, 75, 98, 137, 87, 59,
      50, 208, 116, 29, 200, 128, 82, 13, 138, 107, 53, 42, 34, 123, 203, 65, 174, 111, 101, 19,
      78, 165, 62, 115, 108, 175, 139, 126, 107, 55, 196, 30, 209, 126, 8, 15, 211, 57, 191, 37,
      254, 24, 136, 30, 111, 188, 30, 209, 208, 49, 132, 181, 22, 207, 241, 28, 2, 97, 58, 244,
      179, 190, 120, 249, 174, 99, 6, 215, 232, 173, 1, 20, 216, 224, 191, 247, 78, 223, 101, 153,
      1, 182, 203, 213, 75, 132, 98, 53, 204, 13, 177, 22, 88, 218, 21, 32, 68, 247, 153, 11, 190,
      47, 128, 214, 33, 110, 194, 102, 77, 5, 178, 74, 65, 134, 62, 91, 190, 133, 15, 134, 94, 37,
      247, 205, 51, 224, 152, 15, 13, 13, 233, 189, 206, 100, 131, 222, 5, 70, 182, 231, 176, 167,
      150, 156, 249, 29, 189, 96, 149, 239, 162, 43, 239, 89, 8, 9, 57, 118, 227, 168, 243, 164,
      188, 125, 8, 8, 240, 36, 45, 21, 20, 44, 175,
    ];

    // log i * alpha^i for i = 1..255, where alpha^255 = 1
    // 0 * alpha^{-1} for i=0
    for (i, a) in answers.iter().enumerate() {
      assert_eq!(
        (GF256(LOG_TABLE[i]) * GF256(EXP_TABLE[i % (ORDER - 1)])).0,
        *a
      );
    }
  }

  #[test]
  fn div_works() {
    let answers: [u8; 256] = [
      0, 0, 71, 174, 173, 87, 134, 213, 152, 231, 124, 39, 203, 113, 13, 198, 88, 171, 55, 150,
      177, 227, 25, 225, 227, 180, 157, 225, 252, 122, 88, 161, 45, 87, 148, 78, 40, 165, 74, 134,
      142, 120, 121, 163, 156, 75, 154, 241, 239, 27, 152, 130, 125, 235, 230, 32, 138, 225, 145,
      90, 214, 226, 182, 168, 155, 175, 179, 124, 105, 169, 249, 58, 201, 14, 155, 217, 196, 254,
      201, 143, 229, 12, 178, 24, 100, 226, 163, 234, 177, 36, 75, 106, 114, 208, 162, 63, 235,
      181, 108, 131, 248, 51, 190, 187, 235, 115, 112, 37, 79, 90, 112, 237, 195, 121, 136, 110,
      174, 143, 113, 134, 229, 255, 35, 175, 156, 208, 240, 222, 94, 202, 228, 34, 123, 23, 48, 18,
      122, 114, 75, 243, 212, 139, 56, 132, 157, 119, 219, 170, 236, 11, 51, 86, 224, 221, 142,
      200, 154, 136, 179, 72, 3, 32, 142, 149, 180, 209, 253, 17, 210, 134, 162, 106, 38, 108, 154,
      154, 74, 181, 115, 142, 204, 195, 23, 162, 178, 41, 9, 90, 190, 14, 2, 45, 227, 253, 115, 93,
      155, 244, 83, 219, 11, 196, 167, 241, 33, 60, 103, 69, 181, 189, 145, 130, 174, 137, 65, 65,
      45, 153, 79, 236, 199, 209, 41, 10, 205, 44, 182, 38, 222, 209, 253, 247, 64, 71, 32, 1, 27,
      53, 4, 110, 170, 221, 215, 4, 179, 163, 64, 90, 152, 163, 235, 6, 41, 93, 176, 175,
    ];

    // log i / alpha^i for i = 1..255, where alpha^255 = 1
    // 0 * alpha^{-1} for i=0
    for (i, a) in answers.iter().enumerate() {
      assert_eq!(
        (GF256(LOG_TABLE[i]) / GF256(EXP_TABLE[i % (ORDER - 1)])).0,
        *a
      );
    }
  }

  #[test]
  fn sum_works() {
    let values = vec![GF256(0x53), GF256(0xCA), GF256(0)];
    assert_eq!(values.into_iter().sum::<GF256>().0, 0x99);
  }

  #[test]
  fn product_works() {
    let values = vec![GF256(1), GF256(1), GF256(4)];
    assert_eq!(values.into_iter().product::<GF256>().0, 4);
  }

  #[test]
  fn power_works() {
    let answers = vec![
      vec![GF256(0), GF256(1), GF256(1), GF256(1), GF256(1)],
      vec![GF256(0), GF256(1), GF256(2), GF256(3), GF256(4)],
      vec![GF256(0), GF256(1), GF256(4), GF256(5), GF256(16)],
    ];
    answers.iter().enumerate().for_each(|(i, v)| {
      v.iter()
        .enumerate()
        .for_each(|(j, u)| assert_eq!(GF256(j as u8).pow(i as isize), *u));
    });
    assert_eq!(GF256(4).pow(-1), GF256(71));
    assert_eq!(GF256(4) * GF256(71), GF256(1));
  }
}
