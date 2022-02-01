use bitvec::prelude::*;

pub fn hexdump(bytes: &[u8]) -> String {
  bytes
    .iter()
    .fold("".to_owned(), |s, b| format!("{}{:02X}", s, b))
}

pub fn bitdump(bits: &BitSlice<u8, Msb0>) -> String {
  bits.iter().fold("".to_owned(), |s, b| {
    format!("{}{}", s, if *b { 1 } else { 0 })
  })
}
