use super::{Deduped, GDTrait};
use crate::dict::BasisDict;
use crate::error::*;
use crate::separator::Separator;
use async_trait::async_trait;
use bitvec::prelude::*;
use libecc::{types::*, *};
///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
#[derive(Debug, Clone)]
pub struct ByteGD<C>
where
  C: Code + ByteUnitCode,
{
  pub code: C,
  pub basis_dict: BasisDict<U8VRep>,
  // TODO: separator, sometimes this should be a byte?
  pub chunk_bytelen: usize,
}

impl<C> ByteGD<C>
where
  C: Code + ByteUnitCode,
{
  pub async fn set_error_alignment(&mut self, mat_slice: &[U8VRep]) -> Result<()> {
    ensure!(
      mat_slice.len() == self.code.code_byte_len(),
      "Invalid matrix size"
    );
    self.code.set_precoding(mat_slice)
  }
}

#[async_trait]
impl<C> GDTrait for ByteGD<C>
where
  C: ByteUnitCode + Send + Sync,
{
  fn unit_check(&self) {
    println!("byte unit code");
  }

  async fn dedup(&mut self, buf: &U8SRep) -> Result<Deduped> {
    let last_chunk_pad_bytelen = self.chunk_bytelen - buf.len() % self.chunk_bytelen;
    let mut padded = vec![0u8; last_chunk_pad_bytelen];

    let mut res = BVRep::new();

    let mut byte_ptr = 0usize;
    while byte_ptr <= buf.len() {
      let target = if byte_ptr + self.chunk_bytelen > buf.len() {
        padded.extend_from_slice(&buf[byte_ptr..buf.len()]);
        padded.as_slice()
      } else {
        &buf[byte_ptr..byte_ptr + self.chunk_bytelen]
      };

      // TODO: parallelization here
      let decoded_future = async { self.code.decode(target) };
      let decoded = decoded_future.await?;

      // write result and update dict
      let (sep, id_or_base) = match self.basis_dict.get_id(&decoded.base) {
        Some(bit_id) => (Separator::Deduped.bv(), bit_id),
        None => {
          let _new_id = self.basis_dict.put_base(&decoded.base)?;
          (Separator::AsIs.bv(), BVRep::from_slice(&decoded.base))
        }
      };
      res.extend_from_bitslice(&sep);
      res.extend_from_bitslice(&id_or_base);
      res.extend_from_bitslice(&BVRep::from_slice(&decoded.deviation));

      byte_ptr += self.chunk_bytelen;
    }

    res.force_align();
    Ok(Deduped {
      data: res.as_raw_slice().to_vec(),
      last_chunk_pad_bytelen,
    })
  }
  async fn dup(&mut self, deduped: &Deduped) -> Result<U8VRep> {
    let deduped_bs = BitSlice::from_slice(&deduped.data);

    let u8size = u8::BITS as usize;
    let code_bitlen = self.code.code_byte_len() * u8size;
    let info_bitlen = self.code.info_byte_len() * u8size;
    let dev_bitlen = code_bitlen - info_bitlen;
    let id_bitlen = self.basis_dict.id_bitlen();
    let mut res = U8VRep::new();

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
          let mut bv = deduped_bs[bitptr..bitptr + info_bitlen].to_bitvec().clone();
          bv.force_align();
          let part = bv.as_raw_slice().to_owned();
          let _new_id = self.basis_dict.put_base(&part)?;
          ((&part).to_owned(), info_bitlen)
        }
        Separator::Deduped => {
          let id = deduped_bs[bitptr..bitptr + id_bitlen].to_owned();
          (self.basis_dict.get_base(&id)?, id_bitlen)
        }
      };
      bitptr += step;

      let mut bv = deduped_bs[bitptr..bitptr + dev_bitlen].to_bitvec().clone();
      bv.force_align();
      let dev = bv.as_raw_slice().to_owned();
      bitptr += dev_bitlen;

      let encoded = self.code.encode(&base, &dev)?;

      let target = if bitptr >= deduped_bs.len() - max_bit_pads {
        &encoded.0[deduped.last_chunk_pad_bytelen..]
      } else {
        &encoded.0[..]
      };
      res.extend_from_slice(target);
    }

    Ok(res)
  }
}
