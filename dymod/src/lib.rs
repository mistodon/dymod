#[cfg(debug_assertions)]
extern crate sharedlib;

#[cfg(debug_assertions)]
pub use sharedlib::{Lib, Func, Symbol};


#[macro_export]
macro_rules! dymod
{
    (
        #[path = $libpath: tt]
        pub mod $modname: ident
        {
            $(fn $fnname: ident ( $($argname: ident : $argtype: ty),* ) -> $returntype: ty;)*
        }
    ) =>
    {
        #[cfg(debug_assertions)]
        pub mod $modname
        {
            use ::std::time::SystemTime;
            use ::std::path::PathBuf;
            use $crate::{Lib, Func, Symbol};

            static mut DYLIB: Option<Lib> = None;
            static mut MODIFIED_TIME: Option<SystemTime> = None;

            fn load_lib() -> &'static Lib
            {
                unsafe
                {
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

                    if DYLIB.is_none() || file_changed
                    {
                        // We need to drop the dylib before we reload it.
                        {
                            DYLIB = None;
                        }

                        let lib = Lib::new(&dylibpath).unwrap();
                        DYLIB = Some(lib);
                    }

                    DYLIB.as_ref().unwrap()
                }
            }

            $(
            pub fn $fnname($($argname: $argtype),*) -> $returntype
            {
                let lib = load_lib();
                unsafe
                {
                    let sym: Func<fn($($argtype),*) -> $returntype> =
                        lib.find_func(stringify!($fnname)).unwrap();
                    let symfn = sym.get();
                    symfn($($argname),*)
                }
            }
            )*
        }

        #[cfg(not(debug_assertions))]
        #[path = $libpath]
        pub mod $modname;
    }
}
