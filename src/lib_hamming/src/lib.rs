use bitvec::prelude::*;
mod error;
use error::*;

#[derive(Debug, Clone)]
pub struct Hamming {
  code_len: u32, // n
  info_len: u32, // k
}

impl Hamming {
  pub fn new(code_len: u32, info_len: u32) -> Self {
    assert!(code_len > info_len && info_len > 0);
    assert_eq!((code_len + 1).count_ones(), 1);
    let deg = u32::BITS - (code_len + 1).leading_zeros() - 1;
    assert_eq!(info_len, code_len - deg);

    return Hamming { code_len, info_len };
  }

  pub fn encode(&self, info: &[u8]) -> Result<(), Error> {
    let bitrep = if let Ok(bv) = BitVec::<_, Lsb0>::try_from_slice(info) {
      bv
    } else {
      bail!("Failed to encode to bitvect");
    };

    let coded = BitVec::<u8, Lsb0>::new();

    let chunk_num = if info.len() % self.info_len as usize == 0 {
      bitrep.len() / self.info_len as usize
    } else {
      bitrep.len() / self.info_len as usize + 1
    };

    // adjust info vector size
    for i in 0..chunk_num {
      let to_be_coded = if i != chunk_num - 1 {
        bitrep[i * self.info_len as usize..(i + 1) * self.info_len as usize].to_bitvec()
      } else {
        let mut res = bitrep[i * self.info_len as usize..].to_bitvec();
        if info.len() % self.info_len as usize != 0 {
          for _ in 0..self.info_len as usize - bitrep.len() % self.info_len as usize {
            res.push(false);
          }
        }
        res
      };
      // TODO: calc parity here
      println!("{:?}", to_be_coded);
    }
    // println!("{:?}", coded);

    Ok(())
  }
}

// #[test]
// fn test_7_4() {
//   let lib = Hamming::new(7, 4);
//   lib.encode(&[0u8, 1, 2, 3]); // 32 / 4 = 8
// }
#[test]
fn test_15_11() {
  let lib = Hamming::new(15, 11);
  println!("{:?}", lib);
  lib.encode(&[0u8, 1, 2, 3, 4]); // 40 / 11 = 4
}
