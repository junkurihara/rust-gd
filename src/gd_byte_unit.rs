use super::{Deduped, GDTrait};
use crate::{dict::BasisDict, error::*, separator::Separator};
use async_trait::async_trait;
use bitvec::prelude::*;
use futures::{
  future::join_all,
  stream::{self, StreamExt},
};
use libecc::{types::*, *};
use tokio::task::spawn_blocking;
///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
#[derive(Debug, Clone)]
pub struct ByteGD<C>
where
  C: Code + ByteUnitCode + Clone,
{
  pub code: C,
  pub basis_dict: BasisDict<U8VRep>,
  // TODO: separator, sometimes this should be a byte?
  pub chunk_bytelen: usize,
}

impl<C> ByteGD<C>
where
  C: Code + ByteUnitCode + Clone,
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
  C: ByteUnitCode + Send + Sync + Clone + 'static,
{
  fn unit_check(&self) {
    println!("byte unit code");
  }

  async fn dedup(&mut self, buf: &U8SRep) -> Result<Deduped> {
    let residue = buf.len() % self.chunk_bytelen;
    let (chunk_num, last_chunk_pad_bytelen) = if residue == 0 {
      (buf.len() / self.chunk_bytelen, 0)
    } else {
      (
        (buf.len() - residue) / self.chunk_bytelen + 1,
        self.chunk_bytelen - residue,
      )
    };

    let mut padded = vec![0u8; last_chunk_pad_bytelen];

    let targets = (0..chunk_num)
      .map(|i| {
        let byte_ptr = self.chunk_bytelen * i;
        if i == chunk_num - 1 && residue > 0 {
          padded.extend_from_slice(&buf[byte_ptr..buf.len()]);
          padded.clone()
        } else {
          buf[byte_ptr..byte_ptr + self.chunk_bytelen].to_owned()
        }
      })
      .collect::<Vec<U8VRep>>();

    let decoded_chunks: Vec<_> = join_all(
      stream::iter(targets)
        .map(|v| async {
          let code = self.code.to_owned();
          spawn_blocking(move || code.decode(&v)).await?
        })
        .collect::<Vec<_>>()
        .await,
    )
    .await;

    let mut res = BVRep::new();
    for decoded_wrapped in decoded_chunks {
      let decoded = decoded_wrapped?;
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

    let mut decoded_chunks: Vec<(U8VRep, U8VRep)> = Vec::new();
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

      decoded_chunks.push(match sep {
        Separator::AsIs => {
          let mut bv = deduped_bs[bitptr..bitptr + info_bitlen].to_bitvec().clone();
          bv.force_align();
          let part = bv.as_raw_slice().to_owned();
          let _new_id = self.basis_dict.put_base(&part)?;
          bitptr += info_bitlen;
          let mut bv = deduped_bs[bitptr..bitptr + dev_bitlen].to_bitvec().clone();
          bv.force_align();
          ((&part).to_owned(), bv.as_raw_slice().to_owned())
        }
        Separator::Deduped => {
          let id = deduped_bs[bitptr..bitptr + id_bitlen].to_owned();
          bitptr += id_bitlen;
          let mut bv = deduped_bs[bitptr..bitptr + dev_bitlen].to_bitvec().clone();
          bv.force_align();
          (self.basis_dict.get_base(&id)?, bv.as_raw_slice().to_owned())
        }
      });
      bitptr += dev_bitlen;
    }

    let encoded_chunks: Vec<_> = join_all(
      stream::iter(decoded_chunks)
        .map(|(base, dev)| async {
          let code = self.code.to_owned();
          spawn_blocking(move || code.encode(&base, &dev)).await?
        })
        .collect::<Vec<_>>()
        .await,
    )
    .await;

    let chunk_num = encoded_chunks.len();
    for (i, chunk_wrapped) in encoded_chunks.into_iter().enumerate() {
      let chunk = chunk_wrapped?;
      let target = if i == chunk_num - 1 {
        &chunk.0[deduped.last_chunk_pad_bytelen..]
      } else {
        &chunk.0[..]
      };
      res.extend_from_slice(target);
    }

    Ok(res)
  }
}
