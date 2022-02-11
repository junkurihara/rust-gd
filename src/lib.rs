mod dict;
mod error;
mod separator;
mod types;

use bitvec::prelude::*;
use dict::BasisDict;
use error::*;
use libecc::*;
use separator::Separator;
use types::*;

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
      GDInner::ReedSolomon(x) => (), //x.unit_check(),//TODO:
    }
  }
  pub fn dedup(&mut self, buf: &U8SRep) -> Result<Deduped> {
    match self {
      GDInner::Hamming(x) => x.dedup(buf),
      GDInner::ReedSolomon(x) => {
        //TODO:
        Ok(Deduped {
          data: bitvec![u8, Msb0; 0; 0],
          last_chunk_pad_bytelen: 0,
        })
      } //x.unit_check(),
    }
  }

  pub fn dup(&mut self, deduped: &Deduped) -> Result<U8VRep> {
    match self {
      GDInner::Hamming(x) => x.dup(deduped),
      GDInner::ReedSolomon(x) => {
        //TODO:
        Ok(vec![0u8; 10])
      } //x.unit_check(),
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
  // TODO: dict, implement trait of base dictionary
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
  // TODO: dict, implement trait of base dictionary
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
    assert_eq!(bitptr, deduped.data.len());

    Ok(res.as_raw_slice().to_owned())
  }
}

// impl<C, T> GDTrait for ByteGD<C, T>
// where
//   C: ByteUnitCode,
//   T: std::cmp::Eq + std::hash::Hash + std::clone::Clone,
// {
//   fn unit_check(&self) {
//     println!("byte unit!");
//   }

//   fn dedup(&mut self, buf: &U8SRep) -> Result<Duped> {}
//   fn dup(&mut self, deduped: &Duped) -> Result<U8VRep, Error> {}
// }
// Polymorphism的なことがしんどそうなので、BitGDとByteGDで分けて、GD Traitを実装した方が早そう？
// あるいはGDTraitをGDのTypeに合わせて複数実装する
// struct GD<CodeType, DictType, SepType> {}
// trait GDTrait { fn dup; fn dedup }
// impl GDTrait for GD<T: BitUnitCode, BitDict, BitSep>{ fn dup; fn dedup }
// impl GDTrait for GD<T: ByteUnitCode, ByteDict, ByteSep>{ fn dup; fn dedup }

/////////////////////////////////////////

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn hamming_works() {
    let hamming_deg: usize = 8;
    let hamming_dict_size = 511;
    let mut gd_dedup = GD::Hamming(hamming_deg).setup(hamming_dict_size).unwrap();
    gd_dedup.unit_check();

    // let words: U8VRep = (0..255).into_iter().collect();
    let words = "寿限無(じゅげむ)寿限無(じゅげむ)五劫(ごこう)のすりきれ海砂利(かいじゃり)水魚(すいぎょ)の水行末(すいぎょうまつ) ".to_string().repeat(128).into_bytes();
    println!("org size: {} bits", words.len() * 8);
    let x = gd_dedup.dedup(&words).unwrap();
    println!("deduped size {} bits", x.data.len());
    let mut gd_dup = GD::Hamming(hamming_deg).setup(hamming_dict_size).unwrap();
    let y = gd_dup.dup(&x).unwrap();
    // println!("{:?}", y);
    println!("duped size {} bits", y.len() * 8);
    assert_eq!(y, words);
  }

  #[test]
  fn rs_works() {
    let gd_dedup = GD::ReedSolomon(3, 2).setup(1024).unwrap();
    gd_dedup.unit_check();
    // println!("{:?}", gd);
  }
}
