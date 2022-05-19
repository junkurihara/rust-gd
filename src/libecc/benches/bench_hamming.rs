#![feature(test)]
extern crate test;

use anyhow::Result;
use bitvec::prelude::*;
use libecc::{types::*, *};

const DEG: u32 = 8;
const N_VAR: usize = 255;
const K_VAR: usize = 247;
const N_LOOP: usize = 10;

fn setup_bench(b: &mut test::Bencher, deg: u32) {
  b.iter(|| Hamming::new(deg).unwrap());
}

fn enc_bench(b: &mut test::Bencher, deg: u32) {
  let message: Vec<BVRep> = vec![bitvec![u8, Msb0; 0; K_VAR]; N_LOOP];
  let dev: BVRep = bitvec![u8, Msb0; 0; N_VAR-K_VAR];

  let hamming = Hamming::new(deg).unwrap();

  b.iter(|| {
    message
      .iter()
      .map(|v| hamming.encode(v.as_ref(), dev.as_ref()))
      .collect::<Vec<_>>()
  });
}

#[allow(clippy::needless_collect)]
fn dec_bench(b: &mut test::Bencher, deg: u32) {
  let message: Vec<BVRep> = vec![bitvec![u8, Msb0; 0; K_VAR]; N_LOOP];
  let dev: BVRep = bitvec![u8, Msb0; 0; N_VAR-K_VAR];

  let hamming = Hamming::new(deg).unwrap();

  let _res = message
    .iter()
    .map(|v| hamming.encode(v.as_ref(), dev.as_ref()))
    .collect::<Vec<_>>();
  let encs: Result<Vec<Encoded<BVRep>>> = _res.into_iter().collect();
  let encs = encs
    .unwrap()
    .into_iter()
    .map(|v| v.0)
    .collect::<Vec<BVRep>>();

  b.iter(|| {
    encs
      .iter()
      .map(|v| {
        let m = v.as_ref();
        hamming.decode(m)
      })
      .collect::<Vec<_>>()
  });
}

#[bench]
fn hamming_setup_bench(b: &mut test::Bencher) {
  setup_bench(b, DEG);
}
#[bench]
fn hamming_enc_bench(b: &mut test::Bencher) {
  enc_bench(b, DEG)
}
#[bench]
fn hamming_dec_bench(b: &mut test::Bencher) {
  dec_bench(b, DEG)
}
