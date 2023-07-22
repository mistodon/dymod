#![allow(clippy::needless_doctest_main)]

//! # dymod
//!
//! This crate provides a macro, `dymod!`, which allows you to
//! specify a Rust module which will by dynamically loaded and
//! hotswapped in debug mode, but statically linked in release mode.
//!
//! The current version of this crate is very opinionated about how
//! you structure your dynamic code. Hopefully this can be relaxed a
//! little in future.
//!
//! ## Usage
//!
//! Your dynamically loaded code should be placed in its own
//! sub-crate under your main crate:
//!
//! ```text
//! mycrate/
//!   Cargo.toml
//!   src/
//!     main.rs
//!   subcrate/
//!     Cargo.toml
//!     src/
//!       lib.rs
//! ```
//!
//! Your subcrate must also be compiled as a dylib, so in your
//! `subcrate/Cargo.toml` add:
//!
//! ```toml
//! [lib]
//! crate-type = ["dylib"]
//! ```
//!
//! Now you need to add the code that you want to hotswap. Any
//! functions should be `pub extern "C"` and `#[no_mangle]`.
//!
//! ```rust,no_run
//! // subcrate/src/lib.rs
//! #[no_mangle]
//! pub extern "C" fn count_sheep(sheep: u32) -> &'static str {
//!     match sheep {
//!         0 => "None",
//!         1 => "One",
//!         2 => "Two",
//!         3 => "Many",
//!         _ => "Lots"
//!     }
//! }
//! ```
//!
//! Finally, use the `dymod!` macro to specify your module, along
//! with the functions that are dynamically available from it.
//!
//! ```rust,ignore
//! // src/main.rs
//! use dymod::dymod;
//!
//! dymod! {
//!     #[path = "../subcrate/src/lib.rs"]
//!     pub mod subcrate {
//!         fn count_sheep(sheep: u32) -> &'static str;
//!     }
//! }
//!
//! fn main() {
//!     assert_eq!(subcrate::count_sheep(3), "Many");
//!     loop {
//!         // You can now edit the count_sheep function,
//!         // recompile `subcrate`, and see the result change
//!         // while this code is running.
//!         println!("{}", subcrate::count_sheep(3));
//!     }
//! }
//! ```
//!
//! ## Safety
//!
//! In release mode, the module you specify is linked statically
//! as if it was a regular module, and there should be no safety
//! concerns.
//!
//! However, in debug mode, when the code is being dynamically
//! linked, there are a lot of things that can go wrong. It's
//! possible to accidentally trigger panics, or undefined
//! behaviour.
//!
//! Here is a partial list of what can go wrong in debug mode:
//!
//! -   If you are holding on to data owned by the dylib when the
//!     dylib is hotswapped, you will get undefined behaviour.
//! -   Unless both crates use the system allocator (which is luckily
//!     the default since Rust 1.32.0) then dropping data that
//!     was allocated by the other crate will cause a segfault.
//! -   If you change the definition of a struct on either side of
//!     the boundary, you could get undefined behaviour. (This
//!     includes adding or removing enum variants.)
//! -   If you specify the function signatures incorrectly in the
//!     `dymod!` macro, you will get undefined behaviour.
//!
//! Because of these limitations, it is recommended that you use
//! a small number of dynamic functions, and pass types which are
//! unlikely to change much. For example, at the simplest:
//!
//! ```rust,ignore
//! use dymod::dymod;
//!
//! dymod! {
//!     #[path = "../subcrate/src/lib.rs"]
//!     pub mod subcrate {
//!         fn update_application_state(state: &mut ApplicationState);
//!     }
//! }
//! ```
//!
//! The above function would give you the flexibility to tweak any
//! application state at runtime, but the interface is simple enough
//! that it is easy to maintain.
//!
//! ## Manual reloading
//!
//! By default, the `auto-reload` feature is enabled, which will
//! reload the dynamic library whenever it changes (at the point
//! you try to call one of its functions).
//!
//! If you would prefer to handle reloading yourself, you can disable
//! the feature (`--no-default-features`) and reload it with the
//! `reload()` function of the dymod module.
//!
//! For this same reason, it is currently not possible to define
//! a function named `reload` within your dymod module.

#[cfg(all(target_arch = "wasm32", feature = "force-dynamic"))]
compile_error!("The force-dynamic feature is not supported on WASM targets.");

#[cfg(any(
    feature = "force-dynamic",
    all(
        not(feature = "force-static"),
        not(target_arch = "wasm32"),
        debug_assertions
    )
))]
#[doc(hidden)]
pub use libloading::{Library, Symbol};

#[cfg(any(
    feature = "force-dynamic",
    all(not(feature = "force-static"), debug_assertions)
))]
#[doc(hidden)]
pub const AUTO_RELOAD: bool = cfg!(feature = "auto-reload");

#[cfg(any(
    feature = "force-static",
    all(not(feature = "force-dynamic"), not(debug_assertions))
))]
#[macro_export]
macro_rules! dymod {
    (
        #[path = $libpath: tt]
        pub mod $modname: ident {
            $(fn $fnname: ident ( $($argname: ident : $argtype: ty),* $(,)? ) $(-> $returntype: ty)? ;)*
        }
    ) => {
        #[path = $libpath]
        pub mod $modname;
    };
}

/// Takes a module definition and allows it to be hotswapped in debug
/// mode.
///
/// # Examples
///
/// ```rust,ignore
/// use dymod::dymod;
///
/// dymod! {
///     #[path = "../subcrate/src/lib.rs"]
///     pub mod subcrate {
///         fn count_sheep(sheep: u32) -> &'static str;
///     }
/// }
/// ```
///
/// This creates a module with a single function, `count_sheep`. In
/// debug mode, this function will call into the dynamically loaded
/// `subcrate` dylib. If that crate is recompiled, this function will
/// use the updated code.
///
/// In release mode, this module becomes just a regular Rust module
/// with the contents of `../subcrate/src/lib.rs`. No dynamic linking
/// is performed at all, and the functions are as safe as if they
/// were included normally in this crate.
///
/// # Panics
///
/// Beyond the normal risk of your code panicking, there are a few risks
/// associated with dynamic linking in debug mode. In release mode, static
/// linking occurs and those risks don't apply.
///
/// See the [crate-level documentation](index.html) for more information.
///
/// # Safety
///
/// As above, dynamic linking is inherently unsafe. In release mode,
/// static linking occurs and everything is safe. In debug mode,
/// a variety of undefined behavior is possible.
///
/// See the [crate-level documentation](index.html) for more information.
#[cfg(any(
    feature = "force-dynamic",
    all(not(feature = "force-static"), debug_assertions)
))]
#[macro_export]
macro_rules! dymod {
    (
        #[path = $libpath: tt]
        pub mod $modname: ident {
            $(fn $fnname: ident ( $($argname: ident : $argtype: ty),* $(,)? ) $(-> $returntype: ty)? ;)*
        }
    ) => {
        pub mod $modname {
            use super::*;

            use $crate::{Library, Symbol};

            static mut VERSION: usize = 0;

            static mut DYLIB: Option<Library> = None;
            static mut MODIFIED_TIME: Option<std::time::SystemTime> = None;

            #[cfg(target_os = "macos")]
            const DYLIB_PATH: &'static str = concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/",
                stringify!($modname),
                "/target/debug/lib",
                stringify!($modname),
                ".dylib");

            #[cfg(all(unix, not(target_os = "macos")))]
            const DYLIB_PATH: &'static str = concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/",
                stringify!($modname),
                "/target/debug/lib",
                stringify!($modname),
                ".so");

            #[cfg(windows)]
            const DYLIB_PATH: &'static str = concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/",
                stringify!($modname),
                "/target/debug/",
                stringify!($modname),
                ".dll");

            pub fn reload() {
                let path = unsafe {
                    let delete_old = DYLIB.is_some();

                    // Drop the old
                    DYLIB = None;

                    // Clean up the old
                    if delete_old {
                        let old_path = format!("{}{}", DYLIB_PATH, VERSION - 1);
                        std::fs::remove_file(&old_path).expect("Failed to delete old dylib");
                    }

                    // Create the new
                    let new_path = format!("{}{}", DYLIB_PATH, VERSION);
                    std::fs::copy(DYLIB_PATH, &new_path).expect("Failed to copy new dylib");
                    new_path
                };

                // Clear install name to confuse dyld cache
                #[cfg(target_os = "macos")]
                {
                    let output = std::process::Command::new("install_name_tool")
                        .arg("-id")
                        .arg("")
                        .arg(&path)
                        .output()
                        .expect("Failed to start install_name_tool");

                    assert!(output.status.success(), "install_name_tool failed: {:#?}", output);
                }

                // Load new version
                unsafe {
                    VERSION += 1;
                    DYLIB = Some(Library::new(&path).expect("Failed to load dylib"))
                }
            }

            fn dymod_file_changed() -> bool {
                fn file_changed() -> Result<bool, std::io::Error> {
                    let metadata = std::fs::metadata(&DYLIB_PATH)?;
                    let modified_time = metadata.modified()?;
                    unsafe {
                        let changed = MODIFIED_TIME.is_some() && MODIFIED_TIME != Some(modified_time);
                        MODIFIED_TIME = Some(modified_time);
                        Ok(changed)
                    }
                }

                $crate::AUTO_RELOAD && file_changed().unwrap_or(false)
            }

            fn dymod_get_lib() -> &'static Library {
                unsafe {
                    if DYLIB.is_none() || dymod_file_changed() {
                        reload();
                    }
                    DYLIB.as_ref().unwrap()
                }
            }

            $(
            pub fn $fnname($($argname: $argtype),*) $(-> $returntype)? {
                let lib = dymod_get_lib();
                unsafe {
                    let symbol: Symbol<extern "C" fn($($argtype),*) $(-> $returntype)?> =
                        lib.get(stringify!($fnname).as_bytes()).expect("Failed to get symbol from dylib");
                    symbol($($argname),*)
                }
            }
            )*
        }
    }
}
