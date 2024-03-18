# Usage

you can run directly  with:

```bash
cargo run <path>
```

Actually you can add multiple sources:

```bash
cargo run <path1> <path2> ...
```

**Remember to first create a `result` directory in your current directory. Result files will be copied to that directory, while the original data remain unchanged.**

You can compile with:

```bash
cargo rustc --release -- -C target-cpu=native
```

## Scenario

This simple tool mainly aims to solve one exact problem:

I have a camera and usually take photos with it. I have two SD cards and I use "copy" mode, which means that one same 