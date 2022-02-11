// THIS IS A SAMPLE CODE TO USE GD LIB

use libecc::HexDump;
use rust_gd::*;
use std::io::{self, Read, Write};

const BUFFER_SIZE: usize = 512 * 1024;

fn proc(reader: &mut dyn Read, writer: &mut dyn Write) {
  let mut buf = [0u8; BUFFER_SIZE];
  let deg = 8;
  let dict_size = 1024;
  let mut gd_dedup = GD::Hamming(deg).setup(dict_size).unwrap();
  let mut gd_dup = GD::Hamming(deg).setup(dict_size).unwrap();

  while let Ok(n) = reader.read(&mut buf) {
    if n == 0 {
      break;
    }
    /////////////////////////
    // GD proc here
    if let Ok(deduped) = gd_dedup.dedup(&buf[..n]) {
      // println!("{}", hexdump(deduped.as_raw_slice()));
      let _ = writer.write(
        format!(
          "> Deduped (HexDump):\n> {}\n",
          deduped.data.hexdump().unwrap(),
        )
        .as_bytes(),
      );
      let dup = gd_dup.dup(&deduped);
      let _ = writer
        .write(format!("> Duped:\n> {}\n", String::from_utf8(dup.unwrap()).unwrap()).as_bytes());
      println!("> Compressed {} -> {} (bits)", n * 8, deduped.data.len());
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

  let _ = proc(&mut reader, &mut writer);
}
