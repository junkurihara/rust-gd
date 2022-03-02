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
pub struct BitGD<C>
where
  C: Code + BitUnitCode + Clone,
{
  pub code: C,
  pub basis_dict: BasisDict<BVRep>,
  // TODO: separator, sometimes this should be a byte?
  pub chunk_bytelen: usize,
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
#[async_trait]
impl<C> GDTrait for BitGD<C>
where
  C: BitUnitCode + Send + Sync + Clone + 'static,
{
  fn unit_check(&self) {
    println!("bit unit code");
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

    // Currently Byte Alignment is employed, i.e., message is always in bytes and some padding of < 8bits is applied;
    let code_len = self.code.code_bit_len();
    let code_pad_len = code_len - self.chunk_bytelen * 8;
    let mut padded = vec![0u8; last_chunk_pad_bytelen];
    let targets = (0..chunk_num)
      .map(|i| {
        let byte_ptr = self.chunk_bytelen * i;
        let mut target_bitslice = bitvec![u8, Msb0; 0; code_pad_len];
        target_bitslice.extend_from_raw_slice({
          if i == chunk_num - 1 && residue > 0 {
            padded.extend_from_slice(&buf[byte_ptr..buf.len()]);
            padded.as_slice()
          } else {
            &buf[byte_ptr..byte_ptr + self.chunk_bytelen]
          }
        });
        target_bitslice
      })
      .collect::<Vec<BVRep>>();
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
          (Separator::AsIs.bv(), decoded.base)
        }
      };
      res.extend_from_bitslice(&sep);
      res.extend_from_bitslice(&id_or_base);
      res.extend_from_bitslice(&decoded.deviation);
    }

    res.force_align();
    Ok(Deduped {
      data: res.as_raw_slice().to_vec(),
      last_chunk_pad_bytelen,
    })
  }

  async fn dup(&mut self, deduped: &Deduped) -> Result<U8VRep> {
    let deduped_bs = BitSlice::from_slice(&deduped.data);
    let code_len = self.code.code_bit_len();
    let info_len = self.code.info_bit_len();
    let dev_len = code_len - info_len;
    let id_bitlen = self.basis_dict.id_bitlen();
    let mut res = BVRep::new();

    let mut decoded_chunks: Vec<(BVRep, BVRep)> = Vec::new();
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
          let part = &deduped_bs[bitptr..bitptr + info_len];
          let _new_id = self.basis_dict.put_base(&part.to_bitvec())?;
          bitptr += info_len;
          let dev = deduped_bs[bitptr..bitptr + dev_len].to_bitvec();
          (part.to_bitvec(), dev)
        }
        Separator::Deduped => {
          let id = deduped_bs[bitptr..bitptr + id_bitlen].to_owned();
          bitptr += id_bitlen;
          let dev = deduped_bs[bitptr..bitptr + dev_len].to_bitvec();
          (self.basis_dict.get_base(&id)?, dev)
        }
      });
      bitptr += dev_len;
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
        &chunk.0[code_len - self.chunk_bytelen * 8 + deduped.last_chunk_pad_bytelen * 8..]
      } else {
        &chunk.0[code_len - self.chunk_bytelen * 8..]
      };
      ensure!(target.len() % 8 == 0, "Invalid target in dup");
      res.extend_from_bitslice(target);
    }

    Ok(res.as_raw_slice().to_owned())
  }
}
