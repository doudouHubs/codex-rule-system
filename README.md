# Codex Rule System

`rule-system` is a Codex plugin for managing project-level requirement rules without mixing them into long-term memory.

## Model

- Rule content is project-level only.
- The only storage root is `<project_root>/.codex-rules/`.
- The SQLite database is `<project_root>/.codex-rules/rules.db`.
- Current sessions store selections only: `session_id -> rule_id`.
- Rules have one strong enum module; default module is `global`.
- There are no session rule YAML files and no project rule YAML files in v0.4.

## Capabilities

- `$rule-add`: add project-level rules.
- `$rule-module`: manage module enum entries after Plan Mode / planned module governance.
- `$rule-check`: open the Windows native checklist manager, filter by module, edit project rules, switch `active` / `deprecated`, and choose which rules the current session adopts.
- `$rule-list`: list rules selected by the current session.
- `$rule-update`: update project-level rules that are selected by the current session.
- `$rule-delete`: unselect rules from the current session.
- `$rule-capture`: extract atomic rule candidates from recent conversation context.
- `$rule-display`: decide whether to print full selected rules or a short summary.
- `$rule-scan`: explicitly import legacy YAML rules into the SQLite database.
- `$rule-system`: explain routing, boundaries, and global-prompt migration guidance.

The plugin does not write long-term memory and does not write `project-memory`.

## Storage Schema

SQLite uses four main tables:

- `rule_modules`: strong enum module owner. `global` is built in.
- `rules`: rule identity, title, status, timestamps, pick counters.
- `rule_details`: rule content, module slug, tags JSON, search text.
- `rule_selections`: current-session selection relation keyed by `session_id`.

This is a breaking storage model. Runtime commands ignore old `.codex/session-rules` and `.codex/project-rules` YAML files. Use `$rule-scan` / `rule-system.exe scan` only when you explicitly want to import old YAML data into SQLite.

## Usage

Ask Codex for rule operations naturally:

```text
把这条要求收集成规则：每次修改共享层前先说明影响范围。
新增一个 frontend 模块，先做模块规划。
打开 rule-check checklist 窗口，让我选择当前会话采用哪些规则。
打开 rule-check，只看 frontend 模块。
查看当前会话采用的规则。
当前会话不再采用 rule-12345678。
把已选规则 rule-12345678 的内容改成：命令和路径使用反引号包裹。
扫描旧 .codex 目录里的 YAML 规则，自动分析模块并导入到新的 SQLite 规则库。
```

The action skills include command examples. Those examples resolve script paths relative to each skill's `SKILL.md` source locator, so the plugin works from a local repository, a Codex cache directory, or a GitHub-installed plugin.

## Install

Clone this repository, then install it through your Codex marketplace or plugin UI.

```powershell
git clone https://github.com/doudouHubs/codex-rule-system.git F:\GitlabProjects\codex-rule-system
```

For local development, point a marketplace entry at this repository:

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

Start a new Codex thread after installation so the plugin skills are reloaded.

## Development Checks

```powershell
python -m py_compile scripts/session_rules.py scripts/project_rules.py skills/rule-capture/scripts/rule_capture.py
cargo check --manifest-path tools/rule-system/Cargo.toml
.\scripts\build-rule-system.ps1
.\bin\rule-system.exe --help
.\bin\rule-system.exe scan --project-root <tmp> --json
python <path-to-plugin-creator>\scripts\validate_plugin.py .
```

## License

MIT
