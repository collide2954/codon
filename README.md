# codon

A modal text editor for the terminal. Selection-first editing with a Helix-compatible keymap. Targets Windows, macOS and Linux.

Status: early development.

## Build

```
cargo build --release
```

## Layout

| Crate | Contents |
| --- | --- |
| `crates/core` | Text model. No I/O. |
| `crates/term` | Terminal I/O and rendering. |
| `crates/codon` | The `codon` binary. |

## License

Apache-2.0
