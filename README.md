# dymod

[![Build Status](https://travis-ci.org/mistodon/dymod.svg?branch=master)](https://travis-ci.org/mistodon/dymod)
[![Crates.io](https://img.shields.io/crates/v/dymod.svg)](https://crates.io/crates/dymod)
[![Docs.rs](https://docs.rs/dymod/badge.svg)](https://docs.rs/dymod/0.2.0/dymod/)

This crate provides a macro, `dymod!`, which allows you to specify a Rust module which will by dynamically loaded and hotswapped in debug mode, but statically linked in release mode.

Note that this is _very_ much experimental. The current version of this crate is very opinionated about how you structure your dynamic code. Hopefully this will be relaxed a little in future.


## Usage

Your dynamically loaded code should be placed in its own sub-crate under your main crate:

```
mycrate/
  Cargo.toml
  src/
    main.rs
  subcrate/
    Cargo.toml
    src/
      lib.rs
```

Your subcrate must also be compiled as a dylib, so in your `subcrate/Cargo.toml` add:

```toml
[lib]
crate-type = ["dylib"]
```

Now you need to add the code that you want to hotswap. Any functions should be `pub extern` and `#[no_mangle]`. See the [Limitations]("#limitations") section below for what kind of code you can put here.

```rust
// subcrate/src/lib.rs

#[no_mangle]
pub extern fn count_sheep(sheep: u32) -> &'static str {
    match sheep {
        0 => "None",
        1 => "One",
        2 => "Two",
        3 => "Many",
        _ => "Lots"
    }
}
```

Finally, use the `dymod!` macro to specify your module, along with the functions that are dynamically available from it.

```rust
// mycrate/src/main.rs

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
        // You can now edit the count_sheep function and see
        // the results change while this code is running.
        println!("{}", subcrate::count_sheep(3));
    }
}
```

## Safety

This is really, really unsafe! But only in debug mode. In release mode, the module you specify is linked statically as if it was a regular module, and there should be no safety concerns.

Here is a partial list of what can go wrong in debug mode:

-   If you are holding on to data owned by the dylib when the dylib is hotswapped, you will get undefined behaviour.
-   If you take ownership of any data allocated by the dylib, dropping that data will probably cause a segfault.
-   If you change the definition of a struct on either side of the boundary, you could get undefined behaviour


## Limitations

So as described above, you cannot rely on hotswapping to work if you change struct definitions while your code is running.

You also cannot reliably take ownership of heap allocated data from one side of the boundary to the other.

Generic functions will not work either.

This is again, just a partial list. There really are quite a lot of constraints on what you can do.


## So what is this actually good for then?

I suppose we'll see!

Here are some examples of code that should work and would be useful to hotswap:

```rust
#[no_mangle]
pub extern fn game_update(state: &mut GameState) {
    // Modify game state.
    // No need to return anything problematic.
    unimplemented!()
}

#[no_mangle]
pub extern fn animate_from_to(point_a: [f32; 2], point_b: [f32; 2], time: f32) -> [f32; 2] {
    // Returns only stack-allocated values and so is safe.
    // Specific kind of animation can be changed on the fly.
    unimplemented!()
}

#[no_mangle]
pub extern fn get_configuration() -> Config {
    // Again, returns only stack-allocated values.
    // Allows changing some configuration while running.
    Config {
        // ...
    }
}
```

# Contributing

All PRs welcome! For larger changes, it may be best to raise an issue first for feedback. Thanks!
