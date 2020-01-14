#![allow(clippy::needless_doctest_main)]

//! # dymod
//!
//! This crate provides a macro, `dymod!`, which allows you to
//! specify a Rust module which will by dynamically loaded and
//! hotswapped in debug mode, but statically linked in release mode.
//!
//! Note that this is _very_ much experimental. The current version
//! of this crate is very opinionated about how you structure your
//! dynamic code. Hopefully this will be relaxed a little in future.
//!
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
//! functions should be `pub extern "C"` and `#[no_mangle]`. See the
//! [Limitations]("#limitations") section below for what kind of
//! code you can put here.
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
//! // mycrate/src/main.rs
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
//!         // You can now edit the count_sheep function and see
//!         // the results change while this code is running.
//!         println!("{}", subcrate::count_sheep(3));
//!     }
//! }
//! ```
//!
//! ## Safety
//!
//! This is really, really unsafe! But only in debug mode. In release
//! mode, the module you specify is linked statically as if it was a
//! regular module, and there should be no safety concerns.
//!
//! Here is a partial list of what can go wrong in debug mode:
//!
//! -   If you are holding on to data owned by the dylib when the
//!     dylib is hotswapped, you will get undefined behaviour.
//! -   If you take ownership of any data allocated by the dylib,
//!     dropping that data will probably cause a segfault.
//! -   If you change the definition of a struct on either side of
//!     the boundary, you could get undefined behaviour
//!
//!
//! ## Limitations
//!
//! So as described above, you cannot rely on hotswapping to work if
//! you change struct definitions while your code is running.
//!
//! You also cannot reliably take ownership of heap allocated data
//! from one side of the boundary to the other.
//!
//! Generic functions will not work either.
//!
//! This is again, just a partial list. There really are quite a lot
//! of constraints on what you can do.
//!
//!
//! ## So what is this actually good for then?
//!
//! I suppose we'll see!
//!
//! Here are some examples of code that should work and would be
//! useful to hotswap:
//!
//! ```rust,no_run
//! # struct GameState {};
//! # struct Config {};
//! #[no_mangle]
//! pub extern "C" fn game_update(state: &mut GameState) {
//!     // Modify game state.
//!     // No need to return anything problematic.
//!     unimplemented!()
//! }
//!
//! #[no_mangle]
//! pub extern "C" fn animate_from_to(point_a: [f32; 2], point_b: [f32; 2], time: f32) -> [f32; 2] {
//!     // Returns only stack-allocated values and so is safe.
//!     // Specific kind of animation can be changed on the fly.
//!     unimplemented!()
//! }
//!
//! #[no_mangle]
//! pub extern "C" fn get_configuration() -> Config {
//!     // Again, returns only stack-allocated values.
//!     // Allows changing some configuration while running.
//!     Config
//!     {
//!         // ...
//!     }
//! }
//! ```

#[cfg(any(
    feature = "force-dynamic",
    all(not(feature = "force-static"), debug_assertions)
))]
pub use libloading::{Library, Symbol};

#[cfg(any(
    feature = "force-dynamic",
    all(not(feature = "force-static"), debug_assertions)
))]
pub const AUTO_RELOAD: bool = cfg!(feature = "auto-reload");

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
/// Panics can occur _only_ in debug mode as a result of the various
/// pitfalls of dynamic linking. These can be the result of:
///
/// 1.  Dropping data which was allocated in the other library.
/// 2.  Holding onto references to data that is dropped when the
///     dylib is hotswapped.
/// 3.  Changing the definition of a struct that is passed to or from
///     the other library.
/// 4.  Very many other things.
///
/// These problems should all disappear in release mode, where this
/// code is just statically linked as normal.
///
/// # Safety
///
/// As above, dynamic linking is inherently unsafe. In debug mode,
/// these things can cause a variety of undefined behaviour.
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
    }
}

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
            use std::time::SystemTime;
            use $crate::{Library, Symbol};

            #[cfg(unix)]
            static mut VERSION: usize = 0;

            static mut DYLIB: Option<Library> = None;
            static mut MODIFIED_TIME: Option<SystemTime> = None;

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
                "/target/debug/lib",
                stringify!($modname),
                ".dll");

            #[cfg(unix)]
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

            #[cfg(not(unix))]
            pub fn reload() {
                unsafe {
                    // Drop the old
                    DYLIB = None;

                    // Load new version
                    DYLIB = Some(Library::new(&DYLIB_PATH).expect("Failed to load dylib"))
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
