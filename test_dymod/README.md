# test_dymod

This is mostly just a crate used to run tests for `dymod`. To run those tests, use the `all_tests.sh` script, which will run both debug mode and release mode tests.

To see `dymod` working, try `cargo run` in this directory. You should find that, in debug mode, you can edit and recompile the code in `subcrate` live, while in release mode, it is static.
