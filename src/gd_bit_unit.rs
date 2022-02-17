use super::{Deduped, GDTrait};
use crate::dict::BasisDict;
use crate::error::*;
use crate::separator::Separator;
use bitvec::prelude::*;
use libecc::{types::*, *};
///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
#[derive(Debug, Clone)]
pub struct BitGD<C>
where
  C: Code + BitUnitCode,
{
  pub code: C,
  pub basis_dict: BasisDict<BVRep>,
  // TODO: separator, sometimes this should be a byte?
  pub chunk_bytelen: usize,
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
impl<C> GDTrait for BitGD<C>
where
  C: BitUnitCode,
{
  fn unit_check(&self) {
    println!("bit unit code");
  }

  fn dedup(&mut self, buf: &U8SRep) -> Result<Deduped> {
    // Currently Byte Alignment is employed, i.e., message is always in bytes and some padding of < 8bits is applied;
    let code_len = self.code.code_bit_len();
    let last_chunk_pad_bytelen = self.chunk_bytelen - buf.len() % self.chunk_bytelen;
    let code_pad_len = code_len - self.chunk_bytelen * 8;
    let mut padded = vec![0u8; last_chunk_pad_bytelen];

    let mut res = BVRep::new();

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
      let (sep, id_or_base) = match self.basis_dict.get_id(&decoded.base) {
        Some(bit_id) => (Separator::Deduped.bv(), bit_id),
        None => {
          let _new_id = self.basis_dict.put_base(&decoded.base)?;
          (Separator::AsIs.bv(), decoded.base)
        }
      };
      res.extend_from_bitslice(&sep);
      res.extend_from_bitslice(&id_or_base);
      res.extend_from_bitslice(&decoded.deviation);

      byte_ptr += self.chunk_bytelen;
    }

    res.force_align();
    Ok(Deduped {
      data: res.as_raw_slice().to_vec(),
      last_chunk_pad_bytelen,
    })
  }

  fn dup(&mut self, deduped: &Deduped) -> Result<U8VRep> {
    let deduped_bs = BitSlice::from_slice(&deduped.data);
    let code_len = self.code.code_bit_len();
    let info_len = self.code.info_bit_len();
    let synd_len = code_len - info_len;
    let id_bitlen = self.basis_dict.id_bitlen();
    let mut res = BVRep::new();

    let mut bitptr = 0usize;
    let max_bit_pads = 7usize;
    // max bit pad = 7 bits, if actual bitlen = 9 (0..8), 7bits pad is given.
    // then bitptr = 9 here and deduped_bs.len() = 15
    while bitptr < deduped_bs.len() - max_bit_pads {
      let sep = match deduped_bs[bitptr] {
        false => Separator::AsIs,
        true => Separator::Deduped,
      };
      bitptr += 1;
      let (base, step) = match sep {
        Separator::AsIs => {
          let part = &deduped_bs[bitptr..bitptr + info_len];
          let _new_id = self.basis_dict.put_base(&part.to_bitvec())?;
          (part.to_bitvec(), info_len)
        }
        Separator::Deduped => {
          let part = deduped_bs[bitptr..bitptr + id_bitlen].to_owned();
          (self.basis_dict.get_base(&part)?, id_bitlen)
        }
      };
      bitptr += step;

      let synd = &deduped_bs[bitptr..bitptr + synd_len];
      bitptr += synd_len;

      let encoded = self.code.encode(&base, synd)?;
      let target_bitslice = if bitptr >= deduped_bs.len() - max_bit_pads {
        &encoded.errored[code_len - self.chunk_bytelen * 8 + deduped.last_chunk_pad_bytelen * 8..]
      } else {
        &encoded.errored[code_len - self.chunk_bytelen * 8..]
      };
      ensure!(target_bitslice.len() % 8 == 0, "Invalid target in dup");
      res.extend_from_bitslice(target_bitslice);
    }
    ensure!(
      bitptr >= deduped_bs.len() - max_bit_pads,
      "Invalid deduped data length"
    );

    Ok(res.as_raw_slice().to_owned())
  }
}
