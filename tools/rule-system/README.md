# rule-system

Windows native runtime for `rule-system` project rules.

```powershell
cargo build --release --manifest-path tools/rule-system/Cargo.toml
```

The executable owns SQLite CRUD, module enums, session selections, and the native checklist UI. The internal UI protocol prints:

```json
{"selected_ids":["rule-12345678"],"updates":[],"cancelled":false}
```
