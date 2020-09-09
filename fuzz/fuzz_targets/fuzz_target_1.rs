#![no_main]
use libfuzzer_sys::fuzz_target;

use pretokenizer::PreTokenizer;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let mut pretok = PreTokenizer::new(s);
        while pretok.next().is_some() {}
    }
});