use std::io::{self, Read, Write};

use lib_hamming::hexdump;
use rust_gd::GenDedup;

const BUFFER_SIZE: usize = 512 * 1024;

fn proc(reader: &mut dyn Read, writer: &mut dyn Write) {
  let mut buf = [0u8; BUFFER_SIZE];
  let deg = 8;
  let mut gd_enc = GenDedup::new(deg).unwrap();
  let mut gd_dec = GenDedup::new(deg).unwrap();

  while let Ok(n) = reader.read(&mut buf) {
    if n == 0 {
      break;
    }
    /////////////////////////
    // GD proc here
    if let Ok(deduped) = gd_enc.dedup(&buf[..n]) {
      // println!("{}", hexdump(deduped.as_raw_slice()));
      let _ = writer.write(
        format!(
          "Deduped (HexDump):{} bits -> {} bits\n {}\n",
          n * 8,
          deduped.len(),
          hexdump(deduped.as_raw_slice())
        )
        .as_bytes(),
        // TODO: ここでdecode作ってみる
      );
      let dup = gd_dec.dup(&deduped);
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
