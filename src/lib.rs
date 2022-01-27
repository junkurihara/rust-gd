mod error;

use crate::error::*;
use lib_hamming::*;

#[derive(Debug, Clone)]
pub struct GenDedup {
  code: Hamming,
}

impl GenDedup {
  pub fn new(deg: u32) -> Result<Self, Error> {
    if let Ok(code) = Hamming::new(deg) {
      Ok(GenDedup { code })
    } else {
      bail!("Failed to instantiate associated Hamming code");
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_deg3() {
    let gd = GenDedup::new(3).unwrap();
  }
}
