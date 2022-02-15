mod dict;
mod error;
mod separator;

use bitvec::prelude::*;
use dict::BasisDict;
use error::*;
use libecc::{types::*, *};
use separator::Separator;

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
#[derive(Debug, Clone)]
pub enum GD {
  ReedSolomon(usize, usize),
  Hamming(usize),
}
impl GD {
  pub fn setup(&self, dict_size: usize) -> Result<GDInner> {
    match self {
      GD::ReedSolomon(a, b) => Ok(GDInner::ReedSolomon(ByteGD {
        code: ReedSolomon::new(*a, *b)?,
        basis_dict: BasisDict::<U8VRep>::new(dict_size),
        chunk_bytelen: *a,
      })),

      GD::Hamming(a) => {
        let code = Hamming::new(*a as u32)?;
        ensure!(code.code_bit_len >= 8, "Insufficient code length");
        let chunk_bytelen = (code.code_bit_len - code.code_bit_len % 8) / 8;
        Ok(GDInner::Hamming(BitGD {
          code,
          basis_dict: BasisDict::<BVRep>::new(dict_size),
          chunk_bytelen,
        }))
      }
    }
  }
}
///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
#[derive(Debug, Clone)]
pub enum GDInner {
  ReedSolomon(ByteGD<ReedSolomon>),
  Hamming(BitGD<Hamming>),
}

impl GDInner {
  pub fn unit_check(&self) {
    match &self {
      GDInner::Hamming(x) => x.unit_check(),
      GDInner::ReedSolomon(x) => x.unit_check(),
    }
  }
  pub fn dedup(&mut self, buf: &U8SRep) -> Result<Deduped> {
    match self {
      GDInner::Hamming(x) => x.dedup(buf),
      GDInner::ReedSolomon(x) => x.dedup(buf),
    }
  }

  pub fn dup(&mut self, deduped: &Deduped) -> Result<U8VRep> {
    match self {
      GDInner::Hamming(x) => x.dup(deduped),
      GDInner::ReedSolomon(x) => x.dup(deduped),
    }
  }
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
#[derive(Debug, Clone)]
pub struct BitGD<C>
where
  C: Code + BitUnitCode,
{
  code: C,
  basis_dict: BasisDict<BVRep>,
  // TODO: separator, sometimes this should be a byte?
  chunk_bytelen: usize,
}
///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
#[derive(Debug, Clone)]
pub struct ByteGD<C>
where
  C: Code + ByteUnitCode,
{
  code: C,
  basis_dict: BasisDict<U8VRep>,
  // TODO: separator, sometimes this should be a byte?
  chunk_bytelen: usize,
}
///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
#[derive(Debug, Clone)]
pub struct Deduped {
  pub data: BVRep,
  pub last_chunk_pad_bytelen: usize,
}

pub trait GDTrait {
  fn unit_check(&self);
  fn dedup(&mut self, buf: &U8SRep) -> Result<Deduped>;
  fn dup(&mut self, deduped: &Deduped) -> Result<U8VRep>;
}
///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
impl<C> GDTrait for ByteGD<C>
where
  C: ByteUnitCode,
{
  fn unit_check(&self) {
    println!("byte unit!");
  }

  fn dedup(&mut self, buf: &U8SRep) -> Result<Deduped> {
    let last_chunk_pad_bytelen = self.chunk_bytelen - buf.len() % self.chunk_bytelen;
    let mut padded = vec![0u8; last_chunk_pad_bytelen];

    let mut res = BVRep::new();

    let mut byte_ptr = 0usize;
    while byte_ptr <= buf.len() {
      let mut target = U8VRep::new();
      target.extend_from_slice({
        if byte_ptr + self.chunk_bytelen > buf.len() {
          padded.extend_from_slice(&buf[byte_ptr..buf.len()]);
          padded.as_slice()
        } else {
          &buf[byte_ptr..byte_ptr + self.chunk_bytelen]
        }
      });

      let decoded = self.code.decode(target.as_slice())?;

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

    Ok(Deduped {
      data: res,
      last_chunk_pad_bytelen,
    })
  }
  fn dup(&mut self, deduped: &Deduped) -> Result<U8VRep> {
    let u8size = u8::BITS as usize;
    let code_bitlen = self.code.code_byte_len() * u8size;
    let info_bitlen = self.code.info_byte_len() * u8size;
    let dev_bitlen = code_bitlen - info_bitlen;
    let id_bitlen = self.basis_dict.id_bitlen();
    let mut res = U8VRep::new();

    let mut bitptr = 0usize;
    while bitptr < deduped.data.len() {
      let sep = match deduped.data[bitptr] {
        false => Separator::AsIs,
        true => Separator::Deduped,
      };
      bitptr += 1;

      let (base, step) = match sep {
        Separator::AsIs => {
          let mut bv = deduped.data[bitptr..bitptr + info_bitlen]
            .to_bitvec()
            .clone();
          bv.force_align();
          let part = bv.as_raw_slice().to_owned();
          let _new_id = self.basis_dict.put_base(&part)?;
          ((&part).to_owned(), info_bitlen)
        }
        Separator::Deduped => {
          let id = deduped.data[bitptr..bitptr + id_bitlen].to_owned();
          (self.basis_dict.get_base(&id)?, id_bitlen)
        }
      };
      bitptr += step;

      let mut bv = deduped.data[bitptr..bitptr + dev_bitlen]
        .to_bitvec()
        .clone();
      bv.force_align();
      let dev = bv.as_raw_slice().to_owned();
      bitptr += dev_bitlen;

      let encoded = self.code.encode(&base, &dev)?;
      let target = if bitptr == deduped.data.len() {
        &encoded.errored[deduped.last_chunk_pad_bytelen..]
      } else {
        &encoded.errored[..]
      };
      res.extend_from_slice(target);
    }

    Ok(res)
  }
}
///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
impl<C> GDTrait for BitGD<C>
where
  C: BitUnitCode,
{
  fn unit_check(&self) {
    println!("bit unit!");
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

    Ok(Deduped {
      data: res,
      last_chunk_pad_bytelen,
    })
  }

  fn dup(&mut self, deduped: &Deduped) -> Result<U8VRep> {
    let code_len = self.code.code_bit_len();
    let info_len = self.code.info_bit_len();
    let synd_len = code_len - info_len;
    let id_bitlen = self.basis_dict.id_bitlen();
    let mut res = BVRep::new();

    let mut bitptr = 0usize;
    while bitptr < deduped.data.len() {
      let sep = match deduped.data[bitptr] {
        false => Separator::AsIs,
        true => Separator::Deduped,
      };
      bitptr += 1;
      let (base, step) = match sep {
        Separator::AsIs => {
          let part = &deduped.data[bitptr..bitptr + info_len];
          let _new_id = self.basis_dict.put_base(&part.to_bitvec())?;
          (part.to_bitvec(), info_len)
        }
        Separator::Deduped => {
          let part = deduped.data[bitptr..bitptr + id_bitlen].to_owned();
          (self.basis_dict.get_base(&part)?, id_bitlen)
        }
      };
      bitptr += step;

      let synd = &deduped.data[bitptr..bitptr + synd_len];
      bitptr += synd_len;

      let encoded = self.code.encode(&base, synd)?;
      let target_bitslice = if bitptr == deduped.data.len() {
        &encoded.errored[code_len - self.chunk_bytelen * 8 + deduped.last_chunk_pad_bytelen * 8..]
      } else {
        &encoded.errored[code_len - self.chunk_bytelen * 8..]
      };
      ensure!(target_bitslice.len() % 8 == 0, "Invalid target in dup");
      res.extend_from_bitslice(target_bitslice);
    }
    ensure!(bitptr == deduped.data.len(), "Invalid deduped data length");

    Ok(res.as_raw_slice().to_owned())
  }
}

/////////////////////////////////////////

#[cfg(test)]
mod tests {
  use super::*;
  use rand::Rng;

  // const WORD_STR: &str = "寿限無(じゅげむ)寿限無(じゅげむ)五劫(ごこう)のすりきれ海砂利(かいじゃり)水魚(すいぎょ)の水行末(すいぎょうまつ) ";
  // const WORD_STR: &str = "寿限無(じゅげむ)寿限無(じゅげむ)五劫(ごこう)のすりきれ海砂利(かいじゃり)水魚(すいぎょ)の水行末(すいぎょうまつ) 雲来末(うんらいまつ) 風来末(ふうらいまつ)食(く)う寝(ね)るところに住(す)むところやぶらこうじのぶらこうじパイポパイポパイポのシューリンガンシューリンガンのグーリンダイグーリンダイのポンポコピーのポンポコナの長久命(ちょうきゅうめい)の長助(ちょうすけ)";
  const WORD_STR: &str =
    "寿限無(じゅげむ)寿限無(じゅげむ)五劫(ごこう)のすりきれ海砂利(かいじゃり)padpadpadpadpadpadpadpad"; // Byte alignment is quire needed...

  #[test]
  fn hamming_works() {
    for hamming_deg in 4..11 {
      let hamming_dict_size = 511;

      let mut gd_dedup = GD::Hamming(hamming_deg).setup(hamming_dict_size).unwrap();
      let mut gd_dup = GD::Hamming(hamming_deg).setup(hamming_dict_size).unwrap();
      // gd_dedup.unit_check();

      // println!("Hamimng code deg = {}", hamming_deg);
      let words = WORD_STR.to_string().repeat(128).into_bytes();
      // println!("> org size: {} bits", words.len() * 8);
      let x = gd_dedup.dedup(&words).unwrap();
      // println!("> deduped size {} bits", x.data.len());
      let y = gd_dup.dup(&x).unwrap();
      // println!("> duped size {} bits", y.len() * 8);
      assert_eq!(y, words);
      println!(
        "Hamimng code deg = {} > Deduped rate: {:.2} %",
        hamming_deg,
        100.0 * (x.data.len() as f32) / ((y.len() * 8) as f32)
      );
      // println!()
    }
  }

  const RS_MAX_DICT_BITS: usize = 16;
  const RS_DICT_PARAM: usize = 4;
  const RS_REPEAT: usize = 128;

  #[test]
  fn rs_works() {
    let mut rng = rand::thread_rng();

    for code_len in vec![16, 32, 64, 128].into_iter() {
      for msg_len in 2isize.max(code_len as isize - 8) as usize..code_len {
        let dict_size = (1 << ((code_len - msg_len) * RS_DICT_PARAM).min(RS_MAX_DICT_BITS)) - 1;

        let mut gd_dedup = GD::ReedSolomon(code_len, msg_len).setup(dict_size).unwrap();
        let mut gd_dup = GD::ReedSolomon(code_len, msg_len).setup(dict_size).unwrap();
        // gd_dedup.unit_check();

        let words_org = WORD_STR.to_string().into_bytes().repeat(RS_REPEAT);
        let words: Vec<u8> = words_org
          .into_iter()
          .enumerate()
          .map(|(idx, b)| {
            if idx % RS_REPEAT < msg_len {
              b
            } else {
              let random_pad: u8 = rng.gen();
              b ^ random_pad
            }
          })
          .collect();

        // println!("RS code ({}, {}) over GF(256)", code_len, msg_len);
        // println!("> org size: {} bits", words.len() * 8);
        let x = gd_dedup.dedup(&words).unwrap();
        // println!("> deduped size {} bits", x.data.len());
        let y = gd_dup.dup(&x).unwrap();
        // println!("> duped size {} bits", y.len() * 8);
        assert_eq!(y, words);
        // println!("{:?}", gd);
        println!(
          "RS code ({}, {}) over GF(256) of dict size {} > Deduped rate: {:.2} %",
          code_len,
          msg_len,
          dict_size,
          100.0 * (x.data.len() as f32) / ((y.len() * 8) as f32)
        );
        // println!()
      }
    }
  }
}
