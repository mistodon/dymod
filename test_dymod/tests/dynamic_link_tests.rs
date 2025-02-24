#![cfg(any(
    feature = "force-dynamic",
    feature = "auto-reload",
    all(not(feature = "force-static"), debug_assertions,)
))]

use dymod::dymod;

dymod! {
    #[path = "../subcrate/src/lib.rs"]
    pub mod subcrate {
        fn count_sheep(sheep: u32) -> &'static str;
    }
}

#[test]
#[cfg(not(feature = "auto-reload"))]
fn subcrate_is_dynamically_loaded() {
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

        const UPDATED_LIB: &str = r#"#[unsafe(no_mangle)]
pub extern "C" fn count_sheep(sheep: u32) -> &'static str {
    "Zzzzzzzz..."
}"#;

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

    // Manually reload
    subcrate::reload();

    // Test that it has changed
    {
        assert_eq!(subcrate::count_sheep(0), "Zzzzzzzz...");
        assert_eq!(subcrate::count_sheep(1), "Zzzzzzzz...");
        assert_eq!(subcrate::count_sheep(2), "Zzzzzzzz...");
        assert_eq!(subcrate::count_sheep(3), "Zzzzzzzz...");
        assert_eq!(subcrate::count_sheep(4), "Zzzzzzzz...");
    }
}

#[test]
#[cfg(feature = "auto-reload")]
fn subcrate_is_dynamically_loaded_and_hotswapped() {
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

    // Library should auto-reload

    // Test that it has changed
    {
        assert_eq!(subcrate::count_sheep(0), "Zzzzzzzz...");
        assert_eq!(subcrate::count_sheep(1), "Zzzzzzzz...");
        assert_eq!(subcrate::count_sheep(2), "Zzzzzzzz...");
        assert_eq!(subcrate::count_sheep(3), "Zzzzzzzz...");
        assert_eq!(subcrate::count_sheep(4), "Zzzzzzzz...");
    }
}
