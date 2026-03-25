# Add user-defined agent profiles

Adds an `agents` config map that lets you define named profiles with a command and agent type. Profiles work anywhere an agent name is accepted: `--agent`, `config.agent`, and pane `command` fields.

```yaml
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

```sh
workmux add feature/auth -a cc-work
```

## What changed

- **`config.rs`**: `AgentConfig` struct, `agents: BTreeMap` field on `Config`, `agent_type_override` and `agent_alias` internal fields, alias resolution in `resolve_agent_alias()`, map merging in `merge()`
- **`agent.rs`**: `resolve_profile` now accepts `type_override: Option<&str>`, added `is_known_type()` helper
- **`util.rs`**: Added `resolve_pane_command_with_aliases()` that resolves alias names in pane commands. All functions (`rewrite_agent_command`, `adjust_command`) thread the type override through.
- **`mod.rs`**: Pane setup uses `resolve_pane_command_with_aliases` with the config's agents map. Agent pane detection recognizes aliases.
- **`setup.rs`**: `resolve_pane_configuration` recognizes aliases. Prompt validation recognizes aliases.
- **All multiplexer backends**: `send_keys_to_agent` accepts `agent_type` parameter for correct profile resolution.
- **sandbox/lima**: Type override threaded through for correct container image selection.

## Backward compatibility

- `--agent claude` works identically — no alias match means direct passthrough
- All 787 existing tests pass without modification (only added `None` for the new parameter)
- Empty `agents` map (the default) has zero effect on behavior

## Test plan

- [x] `resolve_profile` with type override returns correct profile
- [x] Type override takes precedence over executable stem
- [x] Invalid type falls back to stem-based resolution
- [x] Config deserializes agents map correctly
- [x] Alias resolution sets command, type override, and alias name
- [x] Non-alias passthrough leaves type override as None
- [x] Agents map merge combines global + project, project wins on conflict
- [x] Alias shadowing a built-in name works correctly
- [x] `cargo test` — 787 tests pass
- [ ] Manual: `workmux add test -a <alias>` launches correct agent
- [ ] Manual: pane `command: <alias>` resolves correctly
