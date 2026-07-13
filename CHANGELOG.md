# Changelog

## 0.4.4

- Removed the right-side current-rule editor from the native `rule-check` window.
- Made the rules table use the full window width, with inline table cells as the only edit surface.
- Kept `Cancel` and `Save and select` fixed at the bottom of the window.

## 0.4.3

- Added inline text editing for the `tags`, `title`, and `content preview` cells in the native `rule-check` table.
- Kept `status` and `module` on table-cell dropdown editors, preserving a single owner for enum edits.
- Synchronized inline text edits back to the right-side editor before saving or selecting rules.

## 0.4.2

- Removed the duplicate right-side `module` and `status` controls from the native `rule-check` editor.
- Kept table-cell dropdowns as the single owner for status/module edits.
- Removed the retired hidden status switch control and Win32 drawing path.
- Expanded the right-side text editor area after removing the duplicate controls.

## 0.4.1

- Added inline dropdown editing for the `status` and `module` cells in the native `rule-check` table.
- Kept checkbox selection semantics unchanged: checkboxes select session rules; cell dropdowns only edit rule metadata.
- Rebuilt `bin/rule-system.exe` with the updated Win32 table-cell editor.

## 0.4.0

- Added `rule_modules` as the strong enum owner for rule modules, with built-in `global`.
- Added `$rule-module` / `module-*` commands for planned module governance.
- Added `module_slug` support to rule details, rule add/update/list/pick/check flows, and search text.
- Added module filtering to `$rule-check`; business module filters include `global` rules.
- Added module editing to the native checklist UI.
- Changed `$rule-scan` to infer modules from legacy YAML rules and auto-create inferred modules deterministically.

## 0.3.0

- Breaking: replaced `.codex/session-rules` and `.codex/project-rules` YAML storage with `<project_root>/.codex-rules/rules.db`.
- Breaking: retired session-level rule content; sessions now persist only selected project rule IDs.
- Added Rust single-exe SQLite owner `bin/rule-system.exe` with `rules`, `rule_details`, and `rule_selections` tables.
- Changed `$rule-check` pick semantics from snapshot copy to current-session selection management.
- Changed `$rule-add` to create project-level rules only; `scope=session` now fails.
- Retired the standalone `rule-picker-win.exe`; the Win32 checklist UI is now part of `rule-system.exe`.
- Added `$rule-scan` / `rule-system.exe scan` for explicit one-time import of legacy YAML rules into SQLite.

## 0.2.8

- Fixed `$rule-add` so Chinese semicolon `；` is always treated as normal text.
- Changed session batch add to recognize only English semicolon `;` separators on both title and content.
- Changed project-scope batch rejection to trigger only on English semicolon `;`, allowing Chinese prose without false failures.

## 0.2.7

- Refined the `$rule-check` Windows picker layout with a clearer title, search section, editor section, and explicit shortcut labels.
- Added live checklist status text showing checked count, visible rule count, total rule count, and the current editing rule.
- Replaced the project rule status text input with a native capsule switch for `active` / `deprecated`.
- Changed `$rule-check pick --ui` to always open the management window and show both `active` and `deprecated` rules instead of pre-filtering to active pick candidates.
- Restored a visible editor target after search/filter refreshes so row focus and checkbox selection stay understandable.

## 0.2.6

- Replaced the `$rule-check` picker list with a native checklist ListView.
- Split row focus from pick selection: checkboxes pick rules, focused rows edit rules.
- Preserved checked rule IDs across search filtering and removed implicit single-match auto-pick.

## 0.2.5

- Upgraded `$rule-check` UI into a Windows native check/edit/pick flow.
- Added picker edit output support for project rule title, content, tags, and status.
- Kept project rule YAML persistence inside `scripts/project_rules.py` as the single storage owner.
- Ensured `pick --ui` still opens an empty-state window instead of failing silently when no active project rules match.

## 0.2.4

- Renamed the project rule operation skill from `$rule-project` to `$rule-check`.
- Retired the `$rule-project` skill entry to keep the autocomplete surface short.

## 0.2.3

- Shortened skill display titles to lowercase `rule-*` names.
- Shortened skill descriptions for cleaner Codex App autocomplete entries.

## 0.2.2

- Removed per-skill `agents/openai.yaml` entries so each `rule-*` skill is exposed only once through `SKILL.md`.
- Kept the 0.2.1 consolidated skill surface unchanged.

## 0.2.1

- Consolidated project rule lifecycle skills into the single `$rule-project` skill to reduce duplicate-looking entries.
- Removed the separate project lifecycle skill entries; project add remains `$rule-add --scope project`.

## 0.2.0

- Merged session and project rule creation under `$rule-add`, with project creation available through `--scope project`.
- Renamed project rule skills to the `rule-*` namespace and retired the old project-first skill names from the plugin source.
- Added the Windows native rule picker and `project_rules.py pick --ui` for fast fuzzy multi-select.

## 0.1.0

- Initial standalone release of the `rule-system` Codex plugin.
- Added current-session rule CRUD, project-shared rule library CRUD, rule capture, rule display, and project rule pick workflows.
