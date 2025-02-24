#![cfg(any(
    feature = "force-static",
    all(
        not(feature = "force-dynamic"),
        not(feature = "auto-reload"),
        not(debug_assertions)
    )
))]

use dymod::dymod;

dymod! {
    #[path = "../subcrate/src/lib.rs"]
    pub mod subcrate {
        fn count_sheep(sheep: u32) -> &'static str;
    }
}

#[test]
fn subcrate_is_statically_linked_and_not_hotswapped() {
    // Test that it works at all
    {
        assert_eq!(subcrate::count_sheep(0), "None");
        assert_eq!(subcrate::count_sheep(1), "One");
        assert_eq!(subcrate::count_sheep(2), "Two");
        assert_eq!(subcrate::count_sheep(3), "Many");
        assert_eq!(subcrate::count_sheep(4), "Lots");
    }

    // Modify the library
    {
        use std::io::Write;

        const UPDATED_LIB: &str = r#"
            #[unsafe(no_mangle)]
            pub extern "C" fn count_sheep(sheep: u32) -> &'static str {
                "Zzzzzzzz..."
            }
            "#;

        let mut file = std::fs::File::create("subcrate/src/lib.rs").expect("Failed to create lib.");

        file.write_all(UPDATED_LIB.as_bytes())
            .expect("Failed to write to lib.");
    }

    // Recompile
    {
        use std::process::Command;

        let _ = Command::new("cargo")
            .arg("build")
            .current_dir("subcrate")
            .output()
            .unwrap();
    }

    // We would have reloaded, if we could have, because of the
    // `auto-reload` feature under `force-static` in this test crate.

    // Test that it has NOT changed
    {
        assert_eq!(subcrate::count_sheep(0), "None");
        assert_eq!(subcrate::count_sheep(1), "One");
        assert_eq!(subcrate::count_sheep(2), "Two");
        assert_eq!(subcrate::count_sheep(3), "Many");
        assert_eq!(subcrate::count_sheep(4), "Lots");
    }
}
