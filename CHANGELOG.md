# Changelog

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
