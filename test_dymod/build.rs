fn main() {
    // Create library
    {
        use std::io::Write;

        const LIB: &str = r#"
        #[no_mangle]
        pub fn count_sheep(sheep: u32) -> &'static str
        {
            match sheep
            {
                0 => "None",
                1 => "One",
                2 => "Two",
                3 => "Many",
                _ => "Lots"
            }
        }
        "#;

        let mut file =
            std::fs::File::create("subcrate/src/lib.rs").expect("Failed to create test lib.");

        file.write_all(LIB.as_bytes())
            .expect("Failed to write test lib source.");
    }

    // Compile it (as a dylib)
    {
        use std::process::Command;

        let _ = Command::new("cargo")
            .arg("build")
            .current_dir("subcrate")
            .output()
            .unwrap();
    }

    println!("cargo:rerun-if-changed=subcrate/src/lib.rs");
}
