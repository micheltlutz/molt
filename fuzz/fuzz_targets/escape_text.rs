#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &str| {
    let escaped = molt::core::escape::escape_text(data);
    assert!(
        !escaped.contains('<'),
        "escape_text emitted a raw '<': {escaped}"
    );
});
