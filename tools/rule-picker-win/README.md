# rule-picker-win

Windows native picker for `rule-system` project rules.

```powershell
cargo build --release --manifest-path tools/rule-picker-win/Cargo.toml
```

The executable reads a JSON array of active project rules and prints:

```json
{"selected_ids":["pr-12345678"],"cancelled":false}
```
