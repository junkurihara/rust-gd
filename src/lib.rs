mod dict;
mod error;
mod separator;

use crate::dict::BaseDict;
use crate::error::*;
use crate::separator::Separator;
use bitvec::prelude::*;
use libecc::*;

#[derive(Debug, Clone)]
pub struct GenDedup {
  code: Hamming,
  chunk_bytelen: usize,
  base_dict: BaseDict,
}

impl GenDedup {
  pub fn new(deg: u32, dict_size: u32) -> Result<Self> {
    let code = Hamming::new(deg)?;
    // TODO: tentative, dict_size should be fixed size like 8bits in most cases
    // TODO: Also must be known to the receiver, so init params must be passed beforehand
    let dict_size = dict_size as usize; //code.info_len;
    let base_dict = BaseDict::new(dict_size);

    let chunk_bytelen = if code.code_len >= 8 {
      (code.code_len - code.code_len % 8) / 8
    } else {
      bail!("Insufficient code length");
    };

    Ok(GenDedup {
      code,
      chunk_bytelen,
      base_dict,
    })
  }

  pub fn dedup(&mut self, buf: &[u8]) -> Result<(BitVec<u8, Msb0>, usize)> {
    // Currently Byte Alignment is employed, i.e., message is always in bytes and some padding of < 8bits is applied;
    // TODO: Or maybe RS or byte-ordered codes are better
    let code_len = self.code.code_len;

    let last_chunk_pad_bytelen = self.chunk_bytelen - buf.len() % self.chunk_bytelen;
    let code_pad_len = code_len - self.chunk_bytelen * 8;
    let mut padded = vec![0u8; last_chunk_pad_bytelen];

    let mut res = BitVec::<u8, Msb0>::new();

    let mut byte_ptr = 0usize;
    while byte_ptr <= buf.len() {
      let mut target_bitslice = bitvec![u8, Msb0; 0; code_pad_len];
      target_bitslice.extend_from_raw_slice({
        if byte_ptr + self.chunk_bytelen > buf.len() {
          padded.extend_from_slice(&buf[byte_ptr..buf.len()]);
          padded.as_slice()
        } else {
          &buf[byte_ptr..byte_ptr + self.chunk_bytelen]
        }
      });
      let decoded = self.code.decode(target_bitslice.as_bitslice())?;

      // write result and update dict
      let (sep, id_or_base) = self.base_dict.get_id_or_base(&decoded.base).unwrap();
      res.extend_from_bitslice(&sep.bv());
      res.extend_from_bitslice(&id_or_base);
      res.extend_from_bitslice(&decoded.deviation);

      byte_ptr += self.chunk_bytelen;
    }

    Ok((res, last_chunk_pad_bytelen))
  }

  pub fn dup(
    &mut self,
    deduped: &BitSlice<u8, Msb0>,
    last_chunk_pad_bytelen: usize,
  ) -> Result<Vec<u8>, Error> {
    let code_len = self.code.code_len;
    let info_len = self.code.info_len;
    let synd_len = code_len - info_len;
    let id_bitlen = self.base_dict.get_id_bitlen();
    let mut res = BitVec::<u8, Msb0>::new();

    let mut bitptr = 0usize;
    while bitptr < deduped.len() {
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
      let base = self.base_dict.get_base(base_or_id, sep)?;
      let synd = &deduped[bitptr..bitptr + synd_len];
      bitptr += synd_len;

      let encoded = self.code.encode(&base, synd)?;
      let target_bitslice = if bitptr == deduped.len() {
        &encoded.errored[code_len - self.chunk_bytelen * 8 + last_chunk_pad_bytelen * 8..]
      } else {
        &encoded.errored[code_len - self.chunk_bytelen * 8..]
      };
      ensure!(target_bitslice.len() % 8 == 0, "Invalid target in dup");
      res.extend_from_bitslice(target_bitslice);
    }
    assert_eq!(bitptr, deduped.len());

    Ok(res.as_raw_slice().to_owned())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_deg_4_to_8() {
    let buf: Vec<u8> = (0u8..255).into_iter().collect();
    for deg in 4..9 {
      let mut gd = GenDedup::new(deg, 1024).unwrap();
      let deduped = gd.dedup(&buf).unwrap();

      let mut rev_gd = GenDedup::new(deg, 1024).unwrap();
      let duped = rev_gd.dup(&deduped.0, deduped.1).unwrap();

      assert_eq!(buf, duped);
    }
  }
}
