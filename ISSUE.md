# User-defined agent profiles

## Problem

Today `--agent` takes a literal command string that serves as both the executable to run and the key to look up agent-specific behavior (prompt injection format, continue flag, etc.). There's no way to define a shorthand like `cc-work` or `cod` that maps to a specific command while inheriting behavior from a known agent type.

This matters when you have multiple configurations of the same agent (e.g., work vs personal Claude installs at different paths) or want shorter aliases for agents you use frequently.

## Proposed solution

Add an `agents` map to the config that defines named profiles:

```yaml
# ~/.config/workmux/config.yaml or .workmux.yaml
agents:
  cc-work:
    command: claude
    type: claude
  cc-personal:
    command: ~/.local/bin/claude-personal
    type: claude
  cod:
    command: codex
    type: codex
```

Each profile has:
- **alias** (map key) — the short name used with `-a`
- **command** — the executable to run
- **type** — which built-in agent profile to inherit behavior from

Usage:
```sh
workmux add my-branch -a cc-work
workmux add my-branch -a cc-personal -a cod  # multi-agent
```

Profiles can also be used in pane configs:
```yaml
panes:
  - command: cc-work
    focus: true
  - command: vim
    split: horizontal
```

Bare agent names like `claude` and `gemini` continue to work exactly as before. The alias is resolved early in config loading — the rest of the system sees the resolved command + type override.

## Details

- Profiles can live in global config (`~/.config/workmux/config.yaml`) and/or project config (`.workmux.yaml`), with project overriding global
- `config.agent` field can also reference an alias: `agent: cc-work`
- The `{{ agent }}` template variable uses the alias name, not the resolved command
- Invalid `type` values warn but don't crash (fall back to default profile behavior)
- Aliases can shadow built-in names if needed
