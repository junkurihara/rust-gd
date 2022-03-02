use libecc::{*, types::*};
use anyhow::Result;
use bitvec::prelude::*;
use futures::{
  future::join_all,
  stream::{self, StreamExt},
};
use std::time::Instant;

const DEG: u32 = 8;
const N_VAR: usize = 255;
const K_VAR: usize = 247;
const N_LOOP: usize = 10;

async fn setup_async(deg: u32) -> Result<Hamming> {
  tokio::task::spawn_blocking(move || Hamming::new(deg)).await?
}

#[tokio::test]
async fn sync_hamming_works() -> Result<()> {
  let before = Instant::now();
  let hamming = Hamming::new(DEG).unwrap();
  let duration = Instant::now().duration_since(before);
  let secs = duration.as_secs() as f64 + duration.subsec_nanos() as f64 / 1000000000.0;
  println!("Sync Set\t {:?}:\t{:.0}/s", duration, N_LOOP as f64 / secs);

  let message: Vec<BVRep> = vec![bitvec![u8, Msb0; 0; K_VAR]; N_LOOP];
  let dev: BVRep = bitvec![u8, Msb0; 0; N_VAR-K_VAR];

  // iter sync
  let before = Instant::now();
  let _res = message
    .iter()
    .map(|v| hamming.encode(v.as_ref(), dev.as_ref()))
    .collect::<Vec<_>>();
  let duration = Instant::now().duration_since(before);
  let secs = duration.as_secs() as f64 + duration.subsec_nanos() as f64 / 1000000000.0;
  println!("Sync Enc\t {:?}:\t{:.0}/s", duration, N_LOOP as f64 / secs);

  let encs: Result<Vec<Encoded<BVRep>>> = _res.into_iter().collect();
  let encs = encs
    .unwrap()
    .into_iter()
    .map(|v| v.0)
    .collect::<Vec<BVRep>>();

  // iter sync
  let before = Instant::now();
  let _res = encs
    .iter()
    .map(|v| {
      let m = v.as_ref();
      hamming.decode(m)
    })
    .collect::<Vec<_>>();
  let duration = Instant::now().duration_since(before);
  let secs = duration.as_secs() as f64 + duration.subsec_nanos() as f64 / 1000000000.0;
  println!("Sync Dec\t {:?}:\t{:.0}/s", duration, N_LOOP as f64 / secs);

  Ok(())
}
#[tokio::test]
async fn async_hamming_works() -> Result<()> {
  let before = Instant::now();
  let hamming = setup_async(DEG).await?;
  let duration = Instant::now().duration_since(before);
  let secs = duration.as_secs() as f64 + duration.subsec_nanos() as f64 / 1000000000.0;
  println!("Async Set\t {:?}:\t{:.0}/s", duration, N_LOOP as f64 / secs);

  let message: Vec<BVRep> = vec![bitvec![u8, Msb0; 0; K_VAR]; N_LOOP];
  let dev: BVRep = bitvec![u8, Msb0; 0; N_VAR-K_VAR];

  // iter async
  let before = Instant::now();
  let inner = stream::iter(message)
    .map(|v| async {
      let hm = hamming.to_owned();
      let d = dev.to_owned();
      tokio::task::spawn_blocking(move || hm.encode(&v, &d)).await?
    })
    .collect::<Vec<_>>()
    .await;
  let _res: Vec<_> = join_all(inner).await;
  let duration = Instant::now().duration_since(before);
  let secs = duration.as_secs() as f64 + duration.subsec_nanos() as f64 / 1000000000.0;
  println!("Async Enc\t {:?}:\t{:.0}/s", duration, N_LOOP as f64 / secs);

  let encs: Result<Vec<Encoded<BVRep>>> = _res.into_iter().collect();
  let encs = encs
    .unwrap()
    .into_iter()
    .map(|v| v.0)
    .collect::<Vec<BVRep>>();

  // iter async
  let before = Instant::now();
  let inner = stream::iter(encs)
    .map(|v| async {
      let hm = hamming.to_owned();
      tokio::task::spawn_blocking(move || hm.decode(&v)).await?
    })
    .collect::<Vec<_>>()
    .await;
  let _res: Vec<_> = join_all(inner).await;
  let duration = Instant::now().duration_since(before);
  let secs = duration.as_secs() as f64 + duration.subsec_nanos() as f64 / 1000000000.0;
  println!("Async Dec\t {:?}:\t{:.0}/s", duration, N_LOOP as f64 / secs);

  // TODO: comparison with join_all

  Ok(())
}
