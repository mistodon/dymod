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
//! Your subcrate must also be compiled as a cdylib, so in your
//! `subcrate/Cargo.toml` add:
//!
//! ```toml
//! [lib]
//! crate-type = ["cdylib"]
//! ```
//!
//! Now you need to add the code that you want to hotswap. Any
//! functions should be `pub` and `#[no_mangle]`. See the
//! [Limitations]("#limitations") section below for what kind of
//! code you can put here.
//!
//! ```rust,no_run
//! // subcrate/src/lib.rs
//! #[no_mangle]
//! pub fn count_sheep(sheep: u32) -> &'static str {
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
//! pub fn game_update(state: &mut GameState) {
//!     // Modify game state.
//!     // No need to return anything problematic.
//!     unimplemented!()
//! }
//!
//! #[no_mangle]
//! pub fn animate_from_to(point_a: [f32; 2], point_b: [f32; 2], time: f32) -> [f32; 2] {
//!     // Returns only stack-allocated values and so is safe.
//!     // Specific kind of animation can be changed on the fly.
//!     unimplemented!()
//! }
//!
//! #[no_mangle]
//! pub fn get_configuration() -> Config {
//!     // Again, returns only stack-allocated values.
//!     // Allows changing some configuration while running.
//!     Config
//!     {
//!         // ...
//!     }
//! }
//! ```

#[cfg(debug_assertions)]
pub use libloading::{Library, Symbol};

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
#[macro_export]
macro_rules! dymod
{
    (
        #[path = $libpath: tt]
        pub mod $modname: ident {
            $(fn $fnname: ident ( $($argname: ident : $argtype: ty),* ) -> $returntype: ty;)*
        }
    ) => {
        #[cfg(debug_assertions)]
        pub mod $modname {
            use std::time::SystemTime;
            use std::path::PathBuf;
            use $crate::{Library, Symbol};

            static mut DYLIB: Option<Library> = None;
            static mut MODIFIED_TIME: Option<SystemTime> = None;

            fn load_lib() -> &'static Library {
                unsafe {
                    let dylibpath = {
                        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
                        path.push(stringify!($modname));
                        path.push("target/debug");
                        path.push(&format!("lib{}.dylib", stringify!($modname)));
                        path
                    };

                    let file_changed = {
                        let metadata = ::std::fs::metadata(&dylibpath).unwrap();
                        let modified_time = metadata.modified().unwrap();
                        let changed = MODIFIED_TIME != Some(modified_time);
                        MODIFIED_TIME = Some(modified_time);
                        changed
                    };

                    if DYLIB.is_none() || file_changed {
                        // We need to drop the dylib before we reload it.
                        {
                            DYLIB = None;
                        }

                        let lib = Library::new(&dylibpath).unwrap();
                        DYLIB = Some(lib);
                    }

                    DYLIB.as_ref().unwrap()
                }
            }

            $(
            pub fn $fnname($($argname: $argtype),*) -> $returntype {
                let lib = load_lib();
                unsafe {
                    let symbol: Symbol<fn($($argtype),*) -> $returntype> =
                        lib.get(stringify!($fnname).as_bytes()).unwrap();
                    symbol($($argname),*)
                }
            }
            )*
        }

        #[cfg(not(debug_assertions))]
        #[path = $libpath]
        pub mod $modname;
    }
}
