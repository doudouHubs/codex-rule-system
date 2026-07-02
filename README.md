# Codex Rule System

`rule-system` is a Codex plugin for managing requirement rules without mixing them into long-term memory.

It provides two layers:

- Session rules: scoped to the current project and current Codex thread.
- Project rules: shared by all conversations in the same project, but only applied after explicit pick.

## Capabilities

- `$rule-add`, `$rule-update`, `$rule-delete`, `$rule-list`: CRUD for current-session rules.
- `$rule-add --scope project`: add stable project-shared rules through the canonical add entry.
- `$rule-capture`: extract atomic rule candidates from recent conversation context.
- `$rule-display`: decide whether to print full rules or a short summary.
- `$rule-check`: list, search, update, delete, or pick project-shared rules, with an optional Windows native picker for picking.
- `$rule-system`: explain routing, boundaries, and global-prompt migration guidance.

## Storage Model

- Session rules: `<project_root>/.codex/session-rules/<session_id>/rules.yaml`
- Project rules: `<project_root>/.codex/project-rules/rules.yaml`

The plugin does not write long-term memory and does not write `project-memory`.

## Install

### Local Development

Clone this repository, then add it as a local marketplace entry or install it through the Codex plugin UI.

```powershell
git clone https://github.com/doudouHubs/codex-rule-system.git F:\GitlabProjects\codex-rule-system
```

For a local marketplace using the standard `./plugins/<name>` convention, place this repository under `plugins/rule-system`, then create the marketplace file at the marketplace root:

```json
{
  "name": "local-rules",
  "interface": {
    "displayName": "Local Rules"
  },
  "plugins": [
    {
      "name": "rule-system",
      "source": {
        "source": "local",
        "path": "./plugins/rule-system"
      },
      "policy": {
        "installation": "AVAILABLE",
        "authentication": "ON_INSTALL"
      },
      "category": "Productivity"
    }
  ]
}
```

### GitHub Distribution

After publishing the repository, install it through the Codex plugin flow that supports GitHub marketplace or local marketplace sources. Start a new Codex thread after installation so the plugin skills are reloaded.

## Usage

Ask Codex for rule operations naturally:

```text
把这条要求收集成规则：每次修改共享层前先说明影响范围。
查看当前会话规则。
把这条项目级规则加入项目规则库：命令和路径使用反引号包裹。
把项目规则库里和输出格式相关的规则拾取到当前会话。
打开规则选择窗口，从项目规则库里手动搜索并拾取规则。
```

The action skills include command examples. Those examples intentionally resolve script paths relative to each skill's `SKILL.md` source locator, so the plugin works from a local repository, a Codex cache directory, or a GitHub-installed plugin.

## Development Checks

```powershell
python -m py_compile scripts/session_rules.py scripts/project_rules.py skills/rule-capture/scripts/rule_capture.py
cargo check --manifest-path tools/rule-picker-win/Cargo.toml
.\scripts\build-rule-picker.ps1
python <path-to-plugin-creator>\scripts\validate_plugin.py .
```

Run skill validation with the Codex skill validator available in your environment.

## License

MIT
