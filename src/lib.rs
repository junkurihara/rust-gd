mod dict;
mod error;
mod gd_bit_unit;
mod gd_byte_unit;
mod separator;

use dict::BasisDict;
use error::*;
use gd_bit_unit::BitGD;
use gd_byte_unit::ByteGD;
use libecc::{types::*, *};

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
pub trait GDTrait {
  fn unit_check(&self);
  fn dedup(&mut self, buf: &U8SRep) -> Result<Deduped>;
  fn dup(&mut self, deduped: &Deduped) -> Result<U8VRep>;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
#[derive(Debug, Clone)]
pub struct Deduped {
  pub data: U8VRep,
  pub last_chunk_pad_bytelen: usize,
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
    let words = WORD_STR.to_string().repeat(128).into_bytes();

    for hamming_deg in 4..11 {
      let hamming_dict_size = 511;

      let mut gd_dedup = GD::Hamming(hamming_deg).setup(hamming_dict_size).unwrap();
      let mut gd_dup = GD::Hamming(hamming_deg).setup(hamming_dict_size).unwrap();
      // gd_dedup.unit_check();

      // println!("Hamimng code deg = {}", hamming_deg);
      // println!("> org size: {} bits", words.len() * 8);
      let x = gd_dedup.dedup(&words).unwrap();
      // println!("> deduped size {} bits", x.data.len());
      let y = gd_dup.dup(&x).unwrap();
      // println!("> duped size {} bits", y.len() * 8);
      assert_eq!(y, words);
      println!(
        "Hamimng code deg = {} > Deduped rate: {:.2} %",
        hamming_deg,
        100.0 * (x.data.len() as f32) / (y.len() as f32)
      );
      // println!()
    }
  }

  const RS_MAX_DICT_BITS: usize = 8;
  const RS_DICT_PARAM: usize = 2;
  const RS_REPEAT: usize = 128;

  #[test]
  fn rs_works() {
    let mut rng = rand::thread_rng();
    let words_org = WORD_STR.to_string().into_bytes().repeat(RS_REPEAT);

    for code_len in vec![3, 4, 8, 16, 32, 64, 128].into_iter() {
      for msg_len in 2isize.max(code_len as isize - 8) as usize..code_len {
        let dict_size = (1 << ((code_len - msg_len) * RS_DICT_PARAM).min(RS_MAX_DICT_BITS)) - 1;

        let mut gd_dedup = GD::ReedSolomon(code_len, msg_len).setup(dict_size).unwrap();
        let mut gd_dup = GD::ReedSolomon(code_len, msg_len).setup(dict_size).unwrap();
        // gd_dedup.unit_check();

        let words: Vec<u8> = words_org
          .clone()
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
          100.0 * (x.data.len() as f32) / (y.len() as f32)
        );
        // println!()
      }
    }
  }
}
