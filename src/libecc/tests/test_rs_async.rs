use libecc::{*, types::*};
use anyhow::Result;

use futures::{
  future::join_all,
  stream::{self, StreamExt},
};
use std::time::Instant;
const N_VAR: usize = 128;
const K_VAR: usize = 126;
const N_LOOP: usize = 10000;

async fn encode_async(
  rs: &ReedSolomon,
  message: &U8SRep,
  dev: &U8SRep,
) -> Result<Encoded<U8VRep>> {
  let rs_clone = rs.to_owned();
  let msg_clone = message.to_owned();
  let dev_clone = dev.to_owned();
  tokio::task::spawn_blocking(move || rs_clone.encode(&msg_clone, &dev_clone)).await?
}

async fn decode_async(rs: &ReedSolomon, data: U8VRep) -> Result<Decoded<U8VRep>> {
  let rs_clone = rs.to_owned();
  let data_clone = data.to_owned();
  tokio::task::spawn_blocking(move || rs_clone.decode(&data_clone)).await?
}

#[allow(clippy::needless_collect)]
#[tokio::test]
async fn sync_rs_works() -> Result<()> {
  let before = Instant::now();
  let rs = ReedSolomon::new(N_VAR, K_VAR).await.unwrap();
  let duration = Instant::now().duration_since(before);
  let secs = duration.as_secs() as f64 + duration.subsec_nanos() as f64 / 1000000000.0;
  println!("Sync Set\t {:?}:\t{:.0}/s", duration, N_LOOP as f64 / secs);

  let message = &[&[0u8; K_VAR]; N_LOOP];
  let dev = &[0u8; N_VAR - K_VAR].to_owned();

  // iter sync
  let before = Instant::now();
  let _res = message
    .iter()
    .map(|v| rs.encode(v.as_ref(), dev.as_ref()))
    .collect::<Vec<_>>();
  let duration = Instant::now().duration_since(before);
  let secs = duration.as_secs() as f64 + duration.subsec_nanos() as f64 / 1000000000.0;
  println!("Sync Enc\t {:?}:\t{:.0}/s", duration, N_LOOP as f64 / secs);

  let encs: Result<Vec<Encoded<U8VRep>>> = _res.into_iter().collect();
  let encs = encs
    .unwrap()
    .into_iter()
    .map(|v| v.0)
    .collect::<Vec<U8VRep>>();

  // iter sync
  let before = Instant::now();
  let _res = encs
    .iter()
    .map(|v| {
      let m = v.as_ref();
      rs.decode(m)
    })
    .collect::<Vec<_>>();
  let duration = Instant::now().duration_since(before);
  let secs = duration.as_secs() as f64 + duration.subsec_nanos() as f64 / 1000000000.0;
  println!("Sync Dec\t {:?}:\t{:.0}/s", duration, N_LOOP as f64 / secs);

  Ok(())
}
#[tokio::test]
async fn async_rs_works() -> Result<()> {
  let before = Instant::now();
  let rs = ReedSolomon::new(N_VAR, K_VAR).await.unwrap();
  let duration = Instant::now().duration_since(before);
  let secs = duration.as_secs() as f64 + duration.subsec_nanos() as f64 / 1000000000.0;
  println!("Async Set\t {:?}:\t{:.0}/s", duration, N_LOOP as f64 / secs);

  let message = &[&[0u8; K_VAR]; N_LOOP];
  let dev = &[0u8; N_VAR - K_VAR].to_owned();

  // iter async
  let before = Instant::now();
  let inner = stream::iter(message)
    .map(|v| encode_async(&rs, v.as_ref(), dev.as_ref()))
    .collect::<Vec<_>>()
    .await;
  let _res: Vec<_> = join_all(inner).await;
  let duration = Instant::now().duration_since(before);
  let secs = duration.as_secs() as f64 + duration.subsec_nanos() as f64 / 1000000000.0;
  println!("Async Enc\t {:?}:\t{:.0}/s", duration, N_LOOP as f64 / secs);

  let encs: Result<Vec<Encoded<U8VRep>>> = _res.into_iter().collect();
  let encs = encs
    .unwrap()
    .into_iter()
    .map(|v| v.0)
    .collect::<Vec<U8VRep>>();

  // iter async
  let before = Instant::now();
  let inner = stream::iter(encs)
    .map(|v| decode_async(&rs, v))
    .collect::<Vec<_>>()
    .await;
  let _res: Vec<_> = join_all(inner).await;
  let duration = Instant::now().duration_since(before);
  let secs = duration.as_secs() as f64 + duration.subsec_nanos() as f64 / 1000000000.0;
  println!("Async Dec\t {:?}:\t{:.0}/s", duration, N_LOOP as f64 / secs);

  // TODO: comparison with join_all

  Ok(())
}
