#![no_main]

use libfuzzer_sys::fuzz_target;
use molt::core::url_policy::is_blocked;
use molt::prelude::*;

fuzz_target!(|data: &str| {
    let stored = Attribute::new("href", data);
    let rendered = a().add_attribute(stored.clone()).render();
    if is_blocked(stored.key(), stored.value()) {
        assert!(
            rendered.contains("href=\"#\""),
            "blocked escaped href was not replaced: {rendered}"
        );
    }

    let raw = Attribute::raw("href", data);
    let raw_rendered = a().add_attribute(raw).render();
    if is_blocked("href", data) {
        assert!(
            raw_rendered.contains("href=\"#\""),
            "blocked raw href was not replaced: {raw_rendered}"
        );
    }
});
