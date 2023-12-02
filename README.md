<h1 align="center">
  <a href="https://github.com/adamws/lukaj"><img src="resources/logo.svg" width="33%"></a>
</h1>
<h4 align="center">

  Interactive diff tool for SVG images.

  [![Crates.io](https://img.shields.io/crates/v/lukaj)](https://crates.io/crates/lukaj)
  [![CI - Main](https://github.com/adamws/lukaj/actions/workflows/build-and-test.yml/badge.svg)](https://github.com/adamws/lukaj/actions/workflows/build-and-test.yml)
  [![License](https://img.shields.io/github/license/adamws/lukaj)](https://github.com/adamws/lukaj/blob/master/LICENSE)
  [![Coverage Status](https://coveralls.io/repos/github/adamws/lukaj/badge.svg?branch=master)](https://coveralls.io/github/adamws/lukaj?branch=master)
</h4>

## Features

Swipe SVG graphics comparison with zoom and drag support.

<p align="center">
  <img src="resources/demo.gif" />
</p>

## Installation

If you're a Rust programmer, lukaj can be installed with `cargo`.
It supports two graphics backends which are enabled/disabled with
[cargo features](https://doc.rust-lang.org/cargo/reference/features.html).

| Feature name         | SVG library                                 | 2D rendering library                                 |
| ---                  | ---                                         | ---                                                  |
| `use-rsvg` (default) | [librsvg](https://crates.io/crates/librsvg) | [cairo](https://www.cairographics.org/)              |
| `use-usvg`           | [resvg](https://crates.io/crates/resvg)     | [tiny-skia](https://github.com/RazrFalcon/tiny-skia) |

Depending on the operating system and selected feature set, lukaj will require
additional development libraries.

<details>
  <summary>On <b>Ubuntu Linux</b> <i>(click to expand)</i></summary>

  1. When using default feature or both (with `cargo --all-features`):

      ```bash
      $ sudo apt-get install build-essential libcairo2-dev libgdk-pixbuf-2.0-dev \
          libglib2.0-dev libpango1.0-dev libsdl2-dev libxml2-dev
      $ cargo install lukaj --all-features
      ```

  2. When using only `use-usvg` feature:

      ```bash
      $ sudo apt-get install libsdl2-dev
      $ cargo install lukaj --no-default-features --features use-usvg
      ```

</details>

<details>
  <summary>On <b>Windows</b> (using <a href="https://www.msys2.org/">MSYS2</a>) <i>(click to expand)</i></summary>

  1. Install MSYS2 and run from it's terminal run:
      - For default feature or both:

        ```bash
        $ pacman -S mingw-w64-x86_64-gtk4 mingw-w64-x86_64-gettext \
            mingw-w64-x86_64-libxml2 mingw-w64-x86_64-pkgconf \
            mingw-w64-x86_64-gcc mingw-w64-x86_64-SDL2
        ```

      - For `use-usvg` feature only:

        ```bash
        $ pacman -S mingw-w64-x86_64-SDL2
        ```

  2. Add mingw binaries path (`C:\msys64\mingw64\bin`) to system `Path`
  3. Change default rust toolchain to `stable-gnu`

      ```bash
      rustup toolchain install stable-gnu
      rustup default stable-gnu
      ```

  4. Run cargo install command

      - For default feature or both:

        ```bash
        $ cargo install lukaj --all-features
        ```

      - For `use-usvg` feature only:

        ```bash
        $ cargo install lukaj --no-default-features --features use-usvg
        ```

  For different setups see this [GUI development with Rust and GTK4](https://gtk-rs.org/gtk4-rs/stable/latest/book/installation_windows.html) guide.
  When running with only `use-usvg` feature see [Rust-SDL2](https://github.com/Rust-SDL2/rust-sdl2) readme
  for different development options.

</details>
</br>

> [!WARNING]
> Precompiled binaries or installation with package managers are not supported yet.

