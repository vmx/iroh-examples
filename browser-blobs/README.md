# iroh blobs browser demo

This example runs iroh-blobs in the browser, compiled to web assembly. It downloads a file without buffering it. Currently it only works in Chromium based browsers.

## Run the browser version:

To build and run it yourself, follow these steps:

```sh
$ cargo install wasm-bindgen-cli
$ rustup target install wasm32-unknown-unknown
$ npm install
$ npm run build
$ npm run serve
```

Then, open [`http://localhost:8080`](http://localhost:8080). The app will print instructions on how to use it.

To build in release mode and apply optimizations to reduce the WASM size, run `npm run build:release` instead.

## Run the CLI version:

Serve up a file:
```sh
cargo run /path/to/file
```

To check whether the files are actually verified try:

```sh
fallocate --length 1M /tmp/1.mib
cargo run /tmp/1.mib
```

Download the file from the browser.

Now run in a different terminal:

```sh
printf '\xFF' | dd of=/tmp/1.mib bs=1 seek=$((0x5)) conv=notrunc status=none
```

Click "download" again in the browser again. It should show an error and the download should be an empty file.

Fix the file again:

```sh
printf '\x00' | dd of=/tmp/1.mib bs=1 seek=$((0x5)) conv=notrunc status=none
```

Now clicking on download should be successful again.


## Navigate the code

This folder contains a single Rust crate that can be compiled to both WebAssembly for the browser and to a command line.

* There code is split between the CLI tool in [`src/node.rs`](src/node.rs) and the WASM-only code in [`src/wasm.rs`](src/wasm.rs).
* The CLI binary is [`src/bin/cli.rs`](src/bin/cli.rs)
* The web app (simple vanilla JS) lives in [`public`].

Running `npm run build` first compiles the crate to WASM, then creates the JavaScript browser bindings with `wasm-bindgen`,
and puts the result into `public/wasm`, from where the WASM is imported in the [`main.js`](public/main.js).
