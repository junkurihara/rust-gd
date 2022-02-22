// THIS IS A SAMPLE CODE TO USE GD LIB

use libecc::HexDump;
use rust_gd::*;
use std::io::{self, Read, Write};

const BUFFER_SIZE: usize = 512 * 1024;

async fn proc(reader: &mut dyn Read, writer: &mut dyn Write) {
  let mut buf = [0u8; BUFFER_SIZE];

  let dict_size = 15;
  let rs_code_len = 5;
  let rs_info_len = 4;
  let mut gd_dedup = GD::ReedSolomon(rs_code_len, rs_info_len)
    .setup(dict_size)
    .await
    .unwrap();
  let mut gd_dup = GD::ReedSolomon(rs_code_len, rs_info_len)
    .setup(dict_size)
    .await
    .unwrap();
  // let trans: Vec<Vec<u8>> = vec![
  //   vec![1u8, 0, 0, 0, 0],
  //   vec![1u8, 1, 1, 1, 5],
  //   vec![1u8, 1, 1, 4, 0],
  //   vec![1u8, 1, 3, 0, 0],
  //   vec![1u8, 2, 0, 0, 0],
  // ];
  // gd_dedup.set_error_alignment(&trans).unwrap();
  // gd_dup.set_error_alignment(&trans).unwrap();
  // let hamming_deg = 8;
  // let mut gd_dedup = GD::Hamming(deg).setup(dict_size).unwrap();
  // let mut gd_dup = GD::Hamming(deg).setup(dict_size).unwrap();

  while let Ok(n) = reader.read(&mut buf) {
    if n == 0 {
      break;
    }
    /////////////////////////
    // GD proc here
    if let Ok(deduped) = gd_dedup.dedup(&buf[..n]).await {
      // println!("{}", hexdump(deduped.as_raw_slice()));
      let _ = writer.write(
        format!(
          "> Deduped (HexDump):\n> {}\n",
          deduped.data.hexdump().unwrap(),
        )
        .as_bytes(),
      );
      let dup = gd_dup.dup(&deduped).await;
      let _ = writer
        .write(format!("> Duped:\n> {}\n", String::from_utf8(dup.unwrap()).unwrap()).as_bytes());
      println!("> Compressed {} -> {} (bytes)", n, deduped.data.len());
    } else {
      panic!("omg");
    }
    /////////////////////////

    // let _ = writer.write(&buf[..n]);
  }
}

fn main() {
  let r = io::stdin();
  let mut reader = r.lock();

  let w = io::stdout();
  let mut writer = w.lock();

  let mut runtime_builder = tokio::runtime::Builder::new_multi_thread();
  runtime_builder.enable_all();
  runtime_builder.thread_name("rust-gd-example");
  let runtime = runtime_builder.build().unwrap();

  runtime.block_on(async move { proc(&mut reader, &mut writer).await });
}
