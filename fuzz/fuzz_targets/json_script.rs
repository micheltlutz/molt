#![no_main]

use libfuzzer_sys::fuzz_target;

// Whatever the input, the output must not hold anything the HTML parser could read as the
// end of a <script> element, a comment, or a character reference, nor the two line
// terminators that were once illegal in a JavaScript string.
fuzz_target!(|data: &str| {
    let escaped = molt::core::escape::escape_json_script(data);
    for forbidden in ['<', '>', '&', '\u{2028}', '\u{2029}'] {
        assert!(
            !escaped.contains(forbidden),
            "escape_json_script emitted {forbidden:?}: {escaped}"
        );
    }
});
