# dymod

[![Build Status](https://travis-ci.org/mistodon/dymod.svg?branch=master)](https://travis-ci.org/mistodon/dymod)
[![Crates.io](https://img.shields.io/crates/v/dymod.svg)](https://crates.io/crates/dymod)
[![Docs.rs](https://docs.rs/dymod/badge.svg)](https://docs.rs/dymod/0.5.0/dymod/)

This crate provides a macro, `dymod!`, which allows you to specify a Rust module which will by dynamically loaded and hotswapped in debug mode, but statically linked in release mode.

The current version of this crate is very opinionated about how you structure your dynamic code. Hopefully this can be relaxed a little in future.

## OS Compatibility

This crate has been tested on macOS (10.14.5), Ubuntu Linux (18.04.1), and Windows 10 (1903). It is however, kind of a weird crate, so I wouldn't be surprised if it failed on some other OSes. Let me know!

## Usage

Your dynamically loaded code should be placed in its own
sub-crate under your main crate:

```text
mycrate/
  Cargo.toml
  src/
    main.rs
  subcrate/
    Cargo.toml
    src/
      lib.rs
```

Your subcrate must also be compiled as a dylib, so in your
`subcrate/Cargo.toml` add:

```toml
[lib]
crate-type = ["dylib"]
```

Now you need to add the code that you want to hotswap. Any
functions should be `pub extern "C"` and `#[unsafe(no_mangle)]`.

```rust,no_run
// subcrate/src/lib.rs
#[unsafe(no_mangle)]
pub extern "C" fn count_sheep(sheep: u32) -> &'static str {
    match sheep {
        0 => "None",
        1 => "One",
        2 => "Two",
        3 => "Many",
        _ => "Lots"
    }
}
```

Finally, use the `dymod!` macro to specify your module, along
with the functions that are dynamically available from it.

```rust,ignore
// src/main.rs
use dymod::dymod;

dymod! {
    #[path = "../subcrate/src/lib.rs"]
    pub mod subcrate {
        fn count_sheep(sheep: u32) -> &'static str;
    }
}

fn main() {
    assert_eq!(subcrate::count_sheep(3), "Many");
    loop {
        // You can now edit the count_sheep function,
        // recompile `subcrate`, and see the result change
        // while this code is running.
        println!("{}", subcrate::count_sheep(3));
    }
}
```

## Safety

In release mode, the module you specify is linked statically
as if it was a regular module, and there should be no safety
concerns.

However, in debug mode, when the code is being dynamically
linked, there are a lot of things that can go wrong. It's
possible to accidentally trigger panics, or undefined
behaviour.

Here is a partial list of what can go wrong in debug mode:

-   If you are holding on to data owned by the dylib when the
    dylib is hotswapped, you will get undefined behaviour.
-   Unless both crates use the system allocator (which is luckily
    the default since Rust 1.32.0) then dropping data that
    was allocated by the other crate will cause a segfault.
-   If you change the definition of a struct on either side of
    the boundary, you could get undefined behaviour. (This
    includes adding or removing enum variants.)
-   If you specify the function signatures incorrectly in the
    `dymod!` macro, you will get undefined behaviour.

Because of these limitations, it is recommended that you use
a small number of dynamic functions, and pass types which are
unlikely to change much. For example, at the simplest:

```rust,ignore
use dymod::dymod;

dymod! {
    #[path = "../subcrate/src/lib.rs"]
    pub mod subcrate {
        fn update_application_state(state: &mut ApplicationState);
    }
}
```

The above function would give you the flexibility to tweak any
application state at runtime, but the interface is simple enough
that it is easy to maintain.

## Manual reloading

By default, the `auto-reload` feature is enabled, which will
reload the dynamic library whenever it changes (at the point
you try to call one of its functions).

If you would prefer to handle reloading yourself, you can disable
the feature (`--no-default-features`) and reload it with the
`reload()` function of the dymod module.

For this same reason, it is currently not possible to define
a function named `reload` within your dymod module.
