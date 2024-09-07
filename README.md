This is an example bot for an in-development programming game. For more information on the game, visit the
 [discord](https://discord.gg/gGypcVEc)

To build this bot you will need the Rust toolchain installed:
[get rustup](https://rustup.rs/)

You will then need an additional tools installed through Rust's buildchain tool cargo:
```
cargo install cargo-component
```

Once that is installed you can build the bot with:
```
cargo component build --release
```

Which should output a WASM file that can be loaded into the game at:
```
target/wasm32-wasip1/release/ai_demo.wasm
```

The first build may take some time but subsequent builds should be very fast.

If you want to develop a Rust bot yourself I would advise installing cargo-watch:
```
cargo install cargo-watch
```

And then running:
```
cargo watch -x 'component build --release'
```

Which will automatically rebuild the bot every time a source file changes and the game will automatically
reload the resulting WASM file when it changes.
