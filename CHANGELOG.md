# Changelog

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
