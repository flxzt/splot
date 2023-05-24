<div align="center">
<img src="misc/splot_logo.svg" width="300"></img>
</div>

# Splot

[![CI](https://github.com/flxzt/splot/actions/workflows/ci.yml/badge.svg)](https://github.com/flxzt/splot/actions/workflows/ci.yml)

A multi-platform serial plotter and monitor.


### Build and run natively

On debian based distros some packages are needed, which can be installed with:

`sudo apt-get install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libspeechd-dev libxkbcommon-dev libssl-dev libudev-dev`

For fedora based distros:

`sudo dnf install clang clang-devel clang-tools-extra speech-dispatcher-devel libxkbcommon-devel pkg-config openssl-devel libxcb-devel fontconfig-devel libudev-devel`

Then build and run the app:

`cargo run --release`

Install with:

`cargo install --path .`

And copy the desktop file to `.local/share/applications`.

### Run web for developing

The app can be compiled to [WASM](https://en.wikipedia.org/wiki/WebAssembly) and published as a web page.

To compile with the Web Serial API enabled, first set the environment variable for it with:

`export RUSTFLAGS=--cfg=web_sys_unstable_apis`.

[Trunk](https://trunkrs.dev/) is used to build for web target.
- Install Trunk with `cargo install --locked trunk`.
- Run `trunk serve` to build and serve on `http://127.0.0.1:8080`. It will use the default `Trunk.toml` configuration
    file and will rebuild automatically if you edit the project.
- Open `http://127.0.0.1:8080/index.html#dev` in a browser. See the warning below.

> `assets/sw.js` script will try to cache our app, and loads the cached version when it cannot connect to server
    allowing your app to work offline (like PWA).
> appending `#dev` to `index.html` will skip this caching, allowing us to load the latest builds during development.

### Deploy web

Execute:

```bash
RUSTFLAGS=--cfg=web_sys_unstable_apis trunk --config Trunk_Release.toml build`
```
