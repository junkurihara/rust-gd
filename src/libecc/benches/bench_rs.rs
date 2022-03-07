#![feature(test)]
extern crate test;

use libecc::{*, types::*};
use anyhow::Result;

const N_VAR: usize = 32;
const K_VAR: usize = 28;
const N_LOOP: usize = 100;


fn get_runtime() -> tokio::runtime::Runtime {
  let mut runtime_builder = tokio::runtime::Builder::new_multi_thread();
  runtime_builder.enable_all();
  runtime_builder.thread_name("bench");
  let runtime = runtime_builder.build().unwrap();
  runtime
}

#[bench]
fn rs_setup_bench(b: &mut test::Bencher){
  let runtime = get_runtime();

  b.iter( || {
    runtime.block_on(async { ReedSolomon::new(N_VAR, K_VAR).await.unwrap() })
  });

}
#[bench]
fn rs_enc_bench(b: &mut test::Bencher){
  let runtime = get_runtime();

  let message = &[&[0u8; K_VAR]; N_LOOP];
  let dev = &[0u8; N_VAR - K_VAR].to_owned();

  let rs = runtime.block_on(async {
    ReedSolomon::new(N_VAR, K_VAR).await.unwrap()
  });

  b.iter( || {
    message
      .iter()
      .map(|v| rs.encode(v.as_ref(), dev.as_ref()))
      .collect::<Vec<_>>()
  });
}
#[bench]
fn rs_dec_bench(b: &mut test::Bencher){
  let runtime = get_runtime();

  let message = &[&[0u8; K_VAR]; N_LOOP];
  let dev = &[0u8; N_VAR - K_VAR].to_owned();

  let rs = runtime.block_on(async {
    ReedSolomon::new(N_VAR, K_VAR).await.unwrap()
  });
  let _res = message
    .iter()
    .map(|v| rs.encode(v.as_ref(), dev.as_ref()))
    .collect::<Vec<_>>();
  let encs: Result<Vec<Encoded<U8VRep>>> = _res.into_iter().collect();
  let encs = encs
    .unwrap()
    .into_iter()
    .map(|v| v.0)
    .collect::<Vec<U8VRep>>();


    b.iter( || {
      encs.iter().map(|v| {
          let m = v.as_ref();
          rs.decode(m)
        })
        .collect::<Vec<_>>()
    })
}
