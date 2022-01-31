mod dict;
mod error;
mod separator;

use std::io::Read;

use crate::dict::BaseDict;
use crate::error::*;
use crate::separator::Separator;
use bitvec::prelude::*;
use lib_hamming::*;

#[derive(Debug, Clone)]
pub struct GenDedup {
  code: Hamming,
  base_dict: BaseDict,
}

impl GenDedup {
  pub fn new(deg: u32) -> Result<Self, Error> {
    if let Ok(code) = Hamming::new(deg) {
      // TODO: tentative, dict_size should be fixed size like 8bits in most cases
      // TODO: Also must be known to the receiver, so init params must be passed beforehand
      let dict_size = code.info_len;
      let base_dict = BaseDict::new(dict_size);
      Ok(GenDedup { code, base_dict })
    } else {
      bail!("Failed to instantiate associated Hamming code");
    }
  }

  pub fn dedup(&mut self, buf: &[u8]) -> Result<(BitVec<u8, Msb0>, usize), Error> {
    let code_len = self.code.code_len;
    let bitbuf = BitSlice::<u8, Msb0>::from_slice(buf);
    let pad_len = code_len - bitbuf.len() % code_len;
    let mut padded = bitvec![u8, Msb0; 0; pad_len];
    let mut res = BitVec::<u8, Msb0>::new();

    let mut bitptr = 0usize;
    while bitptr < bitbuf.len() + pad_len {
      let target_slice = if bitptr + code_len > bitbuf.len() {
        padded.extend_from_bitslice(&bitbuf[bitptr..bitbuf.len()]);
        padded.as_bitslice()
      } else {
        &bitbuf[bitptr..bitptr + code_len]
      };
      let synd = self.code.decode(target_slice);

      // write result and update dict
      let (sep, id_or_base) = self.base_dict.get_id_or_base(&synd.info).unwrap();
      res.extend_from_bitslice(&sep.bv());
      res.extend_from_bitslice(&id_or_base);
      res.extend_from_bitslice(&synd.syndrome);

      bitptr += code_len;
      // println!("{}", target_slice);
    }

    println!("Deduped {} -> {} (bits)", bitbuf.len(), res.len());
    Ok((res, pad_len))
  }

  pub fn dup(&mut self, deduped: &BitSlice<u8, Msb0>, pad_len: usize) -> Result<Vec<u8>, Error> {
    let code_len = self.code.code_len;
    let info_len = self.code.info_len;
    let synd_len = code_len - info_len;
    let id_bitlen = self.base_dict.get_id_bitlen();
    let mut res = BitVec::<u8, Msb0>::new();

    let mut bitptr = 0usize;
    while bitptr < deduped.len() - 1 {
      let sep = match deduped[bitptr] {
        false => Separator::AsIs,
        true => Separator::Deduped,
      };
      bitptr += 1;
      let (base_or_id, step) = match sep {
        Separator::AsIs => (&deduped[bitptr..bitptr + info_len], info_len),
        Separator::Deduped => (&deduped[bitptr..bitptr + id_bitlen], id_bitlen),
      };
      bitptr += step;
      let base = if let Ok(b) = self.base_dict.get_base(base_or_id, sep) {
        b
      } else {
        bail!("Invalid dictionary")
      };
      let synd = &deduped[bitptr..bitptr + synd_len];
      bitptr += synd_len;

      let parity = self.code.encode(&base, synd);
      // println!("{}", parity.erroneous);
      if bitptr == deduped.len() {
        res.extend_from_bitslice(&parity.erroneous[pad_len..]);
      } else {
        res.extend_from_bitslice(&parity.erroneous);
      }
    }
    assert_eq!(bitptr, deduped.len());
    assert_eq!(res.len() % 8, 0);

    Ok(res.as_raw_slice().to_owned())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_deg3() {
    let mut gd = GenDedup::new(3).unwrap();
    let deduped = gd.dedup(&[0u8, 0, 0, 0, 0, 0, 0]).unwrap();
    println!("deduped: {}", deduped.0);
  }
}
