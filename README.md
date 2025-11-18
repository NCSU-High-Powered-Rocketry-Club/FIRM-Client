# FIRM Client

The FIRM Client handles **parsing live FIRM data** and packing it into an easy-to-use format. Its core is written in **Rust**, with libraries for **Python, JavaScript/TypeScript, and (eventually) Arduino**.

---

# 1. Getting Rust + The Project Set Up

The Rust part of the FIRM Client is the core of this repo. It contains the binary parser, data structures, and utilities.

## Install Rust

Follow this guide to get Rust set up in VS Code:

https://www.geeksforgeeks.org/installation-guide/how-to-setup-rust-in-vscode/

## Build the Rust Library

```bash
cargo build
# If this worked, you should see "Finished `dev` profile [unoptimized + debuginfo] target(s)"
```

---

# 2. Building the Python Library (PyPI Package)

We use **maturin** and **uv** to build the Python bindings.

## Install uv

uv is our preferred Python environment + package manager. Install it here:

https://docs.astral.sh/uv/getting-started/installation/

## Python Workflow

First-time setup:

```bash
# Install all Python dependencies (including Maturin)
uv sync --all-extras
```

Build the Python package:

```bash
uv run maturin develop --uv
```

Run the Python test code:

```bash
uv run examples/python/test.py
```

To build a PyPI wheel locally:

```bash
uv run maturin build --release
```

To publish:

1. Make sure you have PyPI credentials set up.
2. Run:

```bash
uv run maturin publish
```

---

# 3. Building the TypeScript / JavaScript Library (npm Package)

The JS/TS bindings wrap the Rust core via wasm-bindgen, bundled through wasm-pack.

## Install npm

npm is our preffered web environment + package manager. Install it and Node.js here:

https://nodejs.org/en/download/

## TypeScript Workflow

First-time setup:

```bash
npm install
```

Build the npm package:

```bash
npm run build
```

To publish:

```bash
# Login to npm
npm login

# Then publish it
npm publish
```

(Ensure the version in `package.json` is bumped before publishing.)

---
