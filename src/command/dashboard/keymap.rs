//! Keymap definitions for dashboard contexts.

use std::collections::{HashMap, HashSet};
use std::mem::Discriminant;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::actions::Action;
use crate::config::KeybindingsConfig;

/// Context for key handling - determines which keymap is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Context {
    DashboardNormal,
    DashboardInput,
    DashboardFilter,
    WorktreeNormal,
    WorktreeFilter,
    DiffNormal,
    Patch,
    Comment,
}

/// User-configurable keymap with override support.
/// Overridden actions have their default keys suppressed; non-overridden actions keep defaults.
pub struct Keymap {
    overrides: HashMap<Context, HashMap<KeyEvent, Action>>,
    overridden_actions: HashMap<Context, HashSet<Discriminant<Action>>>,
}

impl Default for Keymap {
    fn default() -> Self {
        Self::new(&None)
    }
}

impl Keymap {
    pub fn new(config: &Option<KeybindingsConfig>) -> Self {
        let mut overrides: HashMap<Context, HashMap<KeyEvent, Action>> = HashMap::new();
        let mut overridden_actions: HashMap<Context, HashSet<Discriminant<Action>>> =
            HashMap::new();

        let Some(config) = config else {
            return Self {
                overrides,
                overridden_actions,
            };
        };

        let contexts = [
            ("normal", Context::DashboardNormal, config.normal.as_ref()),
            (
                "worktree",
                Context::WorktreeNormal,
                config.worktree.as_ref(),
            ),
            ("diff", Context::DiffNormal, config.diff.as_ref()),
            ("patch", Context::Patch, config.patch.as_ref()),
        ];

        for (ctx_name, ctx, bindings) in contexts {
            let Some(bindings) = bindings else {
                continue;
            };
            let ctx_overrides = overrides.entry(ctx).or_default();
            let ctx_overridden = overridden_actions.entry(ctx).or_default();

            for (action_name, keys) in bindings {
                let Some(action) = parse_action(ctx_name, action_name) else {
                    tracing::warn!(
                        "dashboard.keybindings.{ctx_name}: unknown action '{action_name}'"
                    );
                    continue;
                };

                ctx_overridden.insert(std::mem::discriminant(&action));

                for key_str in keys {
                    let Some(key_event) = parse_key(key_str) else {
                        tracing::warn!(
                            "dashboard.keybindings.{ctx_name}.{action_name}: unknown key '{key_str}'"
                        );
                        continue;
                    };
                    if let Some(existing) = ctx_overrides.get(&key_event) {
                        tracing::warn!(
                            "dashboard.keybindings.{ctx_name}: key '{key_str}' bound to both '{action_name}' and '{:?}', using '{action_name}'",
                            existing
                        );
                    }
                    ctx_overrides.insert(key_event, action.clone());
                }
            }
        }

        Self {
            overrides,
            overridden_actions,
        }
    }

    /// Resolve a key event to an action, checking user overrides first.
    pub fn resolve(&self, ctx: Context, key: KeyEvent) -> Option<Action> {
        // Check user overrides
        if let Some(ctx_overrides) = self.overrides.get(&ctx)
            && let Some(action) = ctx_overrides.get(&key)
        {
            return Some(action.clone());
        }

        // Get default action
        let default = default_action_for_key(ctx, key)?;

        // Suppress if user overrode this action (removed this key from it)
        if let Some(ctx_overridden) = self.overridden_actions.get(&ctx)
            && ctx_overridden.contains(&std::mem::discriminant(&default))
        {
            return None;
        }

        Some(default)
    }
}

/// Parse a key string like "q", "esc", "ctrl+c", "enter" into a KeyEvent.
fn parse_key(s: &str) -> Option<KeyEvent> {
    let s = s.trim().to_lowercase();

    // Handle modifier prefix
    if let Some(rest) = s.strip_prefix("ctrl+") {
        let ch = rest.chars().next()?;
        if rest.len() == 1 {
            return Some(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::CONTROL));
        }
        return None;
    }

    // Special keys
    match s.as_str() {
        "esc" | "escape" => Some(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
        "enter" | "return" => Some(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        "tab" => Some(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)),
        "backspace" | "bksp" => Some(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE)),
        "up" => Some(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)),
        "down" => Some(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)),
        "left" => Some(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)),
        "right" => Some(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)),
        "pageup" | "pgup" => Some(KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE)),
        "pagedown" | "pgdn" => Some(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE)),
        _ => {
            // Single character
            let mut chars = s.chars();
            let ch = chars.next()?;
            if chars.next().is_none() {
                Some(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE))
            } else {
                None
            }
        }
    }
}

/// Parse an action name string to an Action for a given context.
/// Returns None for unknown or parameterized actions.
fn parse_action(ctx: &str, name: &str) -> Option<Action> {
    // Context-independent actions
    let action = match name {
        "show_help" => Action::ShowHelp,
        "switch_tab" => Action::SwitchTab,
        "open_pr" => Action::OpenPr,
        "open_pr_checks" => Action::OpenPrChecks,
        "cycle_color_scheme" => Action::CycleColorScheme,
        "enter_filter_mode" => Action::EnterFilterMode,
        "show_base_branch_picker" => Action::ShowBaseBranchPicker,
        "start_sweep" => Action::StartSweep,
        _ => {
            // Context-dependent actions
            return match ctx {
                "normal" => parse_normal_action(name),
                "worktree" => parse_worktree_action(name),
                "diff" => parse_diff_action(name),
                "patch" => parse_patch_action(name),
                _ => None,
            };
        }
    };
    Some(action)
}

fn parse_normal_action(name: &str) -> Option<Action> {
    match name {
        "quit" => Some(Action::Quit),
        "next" => Some(Action::Next),
        "previous" => Some(Action::Previous),
        "jump_to_selected" => Some(Action::JumpToSelected),
        "jump_to_last" => Some(Action::JumpToLast),
        "peek_selected" => Some(Action::PeekSelected),
        "cycle_sort_mode" => Some(Action::CycleSortMode),
        "toggle_scope_filter" => Some(Action::ToggleScopeFilter),
        "toggle_stale_filter" => Some(Action::ToggleStaleFilter),
        "enter_input_mode" => Some(Action::EnterInputMode),
        "scroll_preview_up" => Some(Action::ScrollPreviewUp),
        "scroll_preview_down" => Some(Action::ScrollPreviewDown),
        "increase_preview_size" => Some(Action::IncreasePreviewSize),
        "decrease_preview_size" => Some(Action::DecreasePreviewSize),
        "load_wip_diff" => Some(Action::LoadWipDiff),
        "send_commit" => Some(Action::SendCommitDashboard),
        "trigger_merge" => Some(Action::TriggerMergeDashboard),
        "kill_selected" => Some(Action::KillSelected),
        _ => None,
    }
}

fn parse_worktree_action(name: &str) -> Option<Action> {
    match name {
        "quit" => Some(Action::Quit),
        "worktree_next" => Some(Action::WorktreeNext),
        "worktree_previous" => Some(Action::WorktreePrevious),
        "jump_to_selected_worktree" => Some(Action::JumpToSelectedWorktree),
        "remove_selected_worktree" => Some(Action::RemoveSelectedWorktree),
        "close_selected_worktree_window" => Some(Action::CloseSelectedWorktreeWindow),
        "cycle_worktree_sort_mode" => Some(Action::CycleWorktreeSortMode),
        "show_project_picker" => Some(Action::ShowProjectPicker),
        _ => None,
    }
}

fn parse_diff_action(name: &str) -> Option<Action> {
    match name {
        "close_diff" => Some(Action::CloseDiff),
        "scroll_up" => Some(Action::ScrollUp),
        "scroll_down" => Some(Action::ScrollDown),
        "scroll_page_up" => Some(Action::ScrollPageUp),
        "scroll_page_down" => Some(Action::ScrollPageDown),
        "toggle_diff_type" => Some(Action::ToggleDiffType),
        "enter_patch_mode" => Some(Action::EnterPatchMode),
        "send_commit" => Some(Action::SendCommitDiff),
        "trigger_merge" => Some(Action::TriggerMergeDiff),
        _ => None,
    }
}

fn parse_patch_action(name: &str) -> Option<Action> {
    match name {
        "exit_patch_mode" => Some(Action::ExitPatchMode),
        "stage_and_next" => Some(Action::StageAndNext),
        "skip_hunk" => Some(Action::SkipHunk),
        "undo_staged_hunk" => Some(Action::UndoStagedHunk),
        "split_hunk" => Some(Action::SplitHunk),
        "start_comment" => Some(Action::StartComment),
        "prev_hunk" => Some(Action::PrevHunk),
        "next_hunk" => Some(Action::NextHunk),
        "scroll_page_up" => Some(Action::ScrollPageUp),
        "scroll_page_down" => Some(Action::ScrollPageDown),
        "send_commit" => Some(Action::SendCommitDiff),
        "trigger_merge" => Some(Action::TriggerMergeDiff),
        _ => None,
    }
}

/// Map a key event to its default action for the given context.
/// This contains the hardcoded default bindings.
fn default_action_for_key(ctx: Context, key: KeyEvent) -> Option<Action> {
    match ctx {
        Context::DashboardNormal => dashboard_normal_key(key),
        Context::DashboardInput => dashboard_input_key(key),
        Context::DashboardFilter => dashboard_filter_key(key),
        Context::WorktreeNormal => worktree_normal_key(key),
        Context::WorktreeFilter => dashboard_filter_key(key),
        Context::DiffNormal => diff_normal_key(key),
        Context::Patch => patch_key(key),
        Context::Comment => comment_key(key),
    }
}

fn dashboard_normal_key(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Char('?') => Some(Action::ShowHelp),
        KeyCode::Char('q') | KeyCode::Esc => Some(Action::Quit),
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Action::Quit),
        KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Action::Next),
        KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Action::Previous)
        }
        KeyCode::Char('j') | KeyCode::Down => Some(Action::Next),
        KeyCode::Char('k') | KeyCode::Up => Some(Action::Previous),
        KeyCode::Enter => Some(Action::JumpToSelected),
        KeyCode::Tab => Some(Action::SwitchTab),
        KeyCode::Backspace => Some(Action::JumpToLast),
        KeyCode::Char('p') => Some(Action::PeekSelected),
        KeyCode::Char('s') => Some(Action::CycleSortMode),
        KeyCode::Char('F') => Some(Action::ToggleScopeFilter),
        KeyCode::Char('f') => Some(Action::ToggleStaleFilter),
        KeyCode::Char('i') => Some(Action::EnterInputMode),
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Action::ScrollPreviewUp)
        }
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Action::ScrollPreviewDown)
        }
        KeyCode::Char('+') | KeyCode::Char('=') => Some(Action::IncreasePreviewSize),
        KeyCode::Char('-') | KeyCode::Char('_') => Some(Action::DecreasePreviewSize),
        KeyCode::Char('d') => Some(Action::LoadWipDiff),
        KeyCode::Char('c') => Some(Action::SendCommitDashboard),
        KeyCode::Char('m') => Some(Action::TriggerMergeDashboard),
        KeyCode::Char('T') => Some(Action::CycleColorScheme),
        KeyCode::Char('/') => Some(Action::EnterFilterMode),
        KeyCode::Char('o') => Some(Action::OpenPr),
        KeyCode::Char('O') => Some(Action::OpenPrChecks),
        KeyCode::Char('b') => Some(Action::ShowBaseBranchPicker),
        KeyCode::Char('X') => Some(Action::KillSelected),
        KeyCode::Char('R') => Some(Action::StartSweep),
        KeyCode::Char(c @ '1'..='9') => Some(Action::JumpToIndex((c as u8 - b'1') as usize)),
        _ => None,
    }
}

fn dashboard_filter_key(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Esc => Some(Action::ClearFilter),
        KeyCode::Enter => Some(Action::AcceptFilter),
        KeyCode::Backspace => Some(Action::FilterDeleteChar),
        KeyCode::Char('?') => Some(Action::ShowHelp),
        KeyCode::Char(c) => Some(Action::FilterAppendChar(c)),
        _ => None,
    }
}

fn worktree_normal_key(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Char('?') => Some(Action::ShowHelp),
        KeyCode::Char('q') => Some(Action::Quit),
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Action::Quit),
        KeyCode::Tab => Some(Action::SwitchTab),
        KeyCode::Char('j') | KeyCode::Down => Some(Action::WorktreeNext),
        KeyCode::Char('k') | KeyCode::Up => Some(Action::WorktreePrevious),
        KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Action::WorktreeNext)
        }
        KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Action::WorktreePrevious)
        }
        KeyCode::Enter => Some(Action::JumpToSelectedWorktree),
        KeyCode::Char('o') => Some(Action::OpenPr),
        KeyCode::Char('O') => Some(Action::OpenPrChecks),
        KeyCode::Char('r') => Some(Action::RemoveSelectedWorktree),
        KeyCode::Char('c') => Some(Action::CloseSelectedWorktreeWindow),
        KeyCode::Char('R') => Some(Action::StartSweep),
        KeyCode::Char('s') => Some(Action::CycleWorktreeSortMode),
        KeyCode::Char('p') => Some(Action::ShowProjectPicker),
        KeyCode::Char('b') => Some(Action::ShowBaseBranchPicker),
        KeyCode::Char('/') => Some(Action::EnterFilterMode),
        KeyCode::Char('T') => Some(Action::CycleColorScheme),
        KeyCode::Char(c @ '1'..='9') => {
            Some(Action::WorktreeJumpToIndex((c as u8 - b'1') as usize))
        }
        _ => None,
    }
}

fn dashboard_input_key(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Esc => Some(Action::ExitInputMode),
        KeyCode::Enter => Some(Action::SendKey("Enter".to_string())),
        KeyCode::Backspace => Some(Action::SendKey("BSpace".to_string())),
        KeyCode::Tab => Some(Action::SendKey("Tab".to_string())),
        KeyCode::Up => Some(Action::SendKey("Up".to_string())),
        KeyCode::Down => Some(Action::SendKey("Down".to_string())),
        KeyCode::Left => Some(Action::SendKey("Left".to_string())),
        KeyCode::Right => Some(Action::SendKey("Right".to_string())),
        KeyCode::Char(c) => Some(Action::SendKey(c.to_string())),
        _ => None,
    }
}

fn diff_normal_key(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Char('?') => Some(Action::ShowHelp),
        KeyCode::Esc | KeyCode::Char('q') => Some(Action::CloseDiff),
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Action::Quit),
        KeyCode::Char('j') | KeyCode::Down => Some(Action::ScrollDown),
        KeyCode::Char('k') | KeyCode::Up => Some(Action::ScrollUp),
        KeyCode::PageDown => Some(Action::ScrollPageDown),
        KeyCode::PageUp => Some(Action::ScrollPageUp),
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Action::ScrollPageDown)
        }
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Action::ScrollPageUp)
        }
        KeyCode::Tab => Some(Action::ToggleDiffType),
        KeyCode::Char('a') => Some(Action::EnterPatchMode),
        KeyCode::Char('c') => Some(Action::SendCommitDiff),
        KeyCode::Char('m') => Some(Action::TriggerMergeDiff),
        _ => None,
    }
}

fn patch_key(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Char('?') => Some(Action::ShowHelp),
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Action::Quit),
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Action::ScrollPageDown)
        }
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Action::ScrollPageUp)
        }
        KeyCode::Char('y') => Some(Action::StageAndNext),
        KeyCode::Char('n') => Some(Action::SkipHunk),
        KeyCode::Char('u') => Some(Action::UndoStagedHunk),
        KeyCode::Char('s') => Some(Action::SplitHunk),
        KeyCode::Char('o') => Some(Action::StartComment),
        KeyCode::Char('k') | KeyCode::Up => Some(Action::PrevHunk),
        KeyCode::Char('j') | KeyCode::Down => Some(Action::NextHunk),
        KeyCode::Char('c') => Some(Action::SendCommitDiff),
        KeyCode::Char('m') => Some(Action::TriggerMergeDiff),
        KeyCode::Esc | KeyCode::Char('q') => Some(Action::ExitPatchMode),
        _ => None,
    }
}

fn comment_key(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Esc => Some(Action::CancelComment),
        KeyCode::Enter => Some(Action::SendComment),
        KeyCode::Backspace => Some(Action::DeleteChar),
        KeyCode::Char(c) => Some(Action::AppendChar(c)),
        _ => None,
    }
}

/// Format a KeyEvent as a display string for help text.
fn format_key(key: &KeyEvent) -> String {
    if key.modifiers.contains(KeyModifiers::CONTROL)
        && let KeyCode::Char(c) = key.code
    {
        return format!("C-{c}");
    }
    match key.code {
        KeyCode::Char(c) => c.to_string(),
        KeyCode::Esc => "Esc".to_string(),
        KeyCode::Enter => "Enter".to_string(),
        KeyCode::Tab => "Tab".to_string(),
        KeyCode::Backspace => "Bksp".to_string(),
        KeyCode::Up => "Up".to_string(),
        KeyCode::Down => "Down".to_string(),
        KeyCode::Left => "Left".to_string(),
        KeyCode::Right => "Right".to_string(),
        KeyCode::PageUp => "PgUp".to_string(),
        KeyCode::PageDown => "PgDn".to_string(),
        _ => "?".to_string(),
    }
}

/// Default help data: (action_discriminant_for_override_check, default_key_str, description).
/// The action is used to check if the user has overridden keys for it.
struct HelpEntry {
    action: Option<Action>,
    default_keys: &'static str,
    description: &'static str,
}

impl HelpEntry {
    fn new(action: Action, default_keys: &'static str, description: &'static str) -> Self {
        Self {
            action: Some(action),
            default_keys,
            description,
        }
    }
    fn static_row(default_keys: &'static str, description: &'static str) -> Self {
        Self {
            action: None,
            default_keys,
            description,
        }
    }
}

impl Keymap {
    /// Get help rows for a context, reflecting any user overrides.
    pub fn help_rows(&self, ctx: Context) -> Vec<(String, String)> {
        let entries = default_help_entries(ctx);
        let ctx_overrides = self.overrides.get(&ctx);
        let ctx_overridden = self.overridden_actions.get(&ctx);

        entries
            .into_iter()
            .filter_map(|entry| {
                let Some(action) = &entry.action else {
                    // Static row (not remappable) — always show
                    return Some((
                        entry.default_keys.to_string(),
                        entry.description.to_string(),
                    ));
                };

                let disc = std::mem::discriminant(action);

                if ctx_overridden.is_some_and(|s| s.contains(&disc)) {
                    // User overrode this action — build key string from overrides
                    if let Some(overrides) = ctx_overrides {
                        let keys: Vec<String> = overrides
                            .iter()
                            .filter(|(_, a)| std::mem::discriminant(*a) == disc)
                            .map(|(k, _)| format_key(k))
                            .collect();
                        if keys.is_empty() {
                            return None; // Action has no keys — hide from help
                        }
                        return Some((keys.join("/"), entry.description.to_string()));
                    }
                    return None;
                }

                Some((
                    entry.default_keys.to_string(),
                    entry.description.to_string(),
                ))
            })
            .collect()
    }
}

fn default_help_entries(ctx: Context) -> Vec<HelpEntry> {
    match ctx {
        Context::DashboardNormal => vec![
            HelpEntry::new(Action::ShowHelp, "?", "Show help"),
            HelpEntry::new(Action::Quit, "q/Esc", "Quit"),
            HelpEntry::new(Action::Next, "j/k/C-n/C-p", "Navigate up/down"),
            HelpEntry::new(Action::JumpToSelected, "Enter", "Jump to agent"),
            HelpEntry::new(Action::SwitchTab, "Tab", "Switch view"),
            HelpEntry::new(Action::JumpToLast, "Bksp", "Last agent"),
            HelpEntry::new(Action::PeekSelected, "p", "Peek agent (keep popup)"),
            HelpEntry::new(Action::CycleSortMode, "s", "Cycle sort mode"),
            HelpEntry::new(Action::ToggleScopeFilter, "F", "Toggle session filter"),
            HelpEntry::new(Action::ToggleStaleFilter, "f", "Toggle stale filter"),
            HelpEntry::new(Action::EnterInputMode, "i", "Enter input mode"),
            HelpEntry::new(Action::ScrollPreviewUp, "Ctrl+u/d", "Scroll preview"),
            HelpEntry::new(Action::IncreasePreviewSize, "+/-", "Resize preview"),
            HelpEntry::new(Action::LoadWipDiff, "d", "View diff"),
            HelpEntry::new(Action::SendCommitDashboard, "c", "Commit changes"),
            HelpEntry::new(Action::TriggerMergeDashboard, "m", "Merge branch"),
            HelpEntry::new(Action::ShowBaseBranchPicker, "b", "Change base branch"),
            HelpEntry::new(Action::OpenPr, "o", "Open PR in browser"),
            HelpEntry::new(Action::OpenPrChecks, "O", "Open PR checks in browser"),
            HelpEntry::new(Action::KillSelected, "X", "Kill agent"),
            HelpEntry::new(Action::StartSweep, "R", "Sweep cleanup"),
            HelpEntry::new(Action::EnterFilterMode, "/", "Filter agents"),
            HelpEntry::new(Action::CycleColorScheme, "T", "Cycle theme"),
            HelpEntry::static_row("1-9", "Quick jump"),
        ],
        Context::DashboardInput => vec![
            HelpEntry::static_row("Esc", "Exit input mode"),
            HelpEntry::static_row("<keys>", "Send to agent"),
        ],
        Context::DashboardFilter | Context::WorktreeFilter => vec![
            HelpEntry::static_row("Enter", "Accept filter"),
            HelpEntry::static_row("Esc", "Clear filter"),
            HelpEntry::static_row("<type>", "Filter text"),
        ],
        Context::WorktreeNormal => vec![
            HelpEntry::new(Action::ShowHelp, "?", "Show help"),
            HelpEntry::new(Action::Quit, "q/Esc", "Quit"),
            HelpEntry::new(Action::WorktreeNext, "j/k/C-n/C-p", "Navigate up/down"),
            HelpEntry::new(Action::JumpToSelectedWorktree, "Enter", "Jump to worktree"),
            HelpEntry::new(Action::SwitchTab, "Tab", "Switch to agents"),
            HelpEntry::new(Action::OpenPr, "o", "Open PR in browser"),
            HelpEntry::new(Action::OpenPrChecks, "O", "Open PR checks in browser"),
            HelpEntry::new(Action::RemoveSelectedWorktree, "r", "Remove worktree"),
            HelpEntry::new(Action::CloseSelectedWorktreeWindow, "c", "Close mux window"),
            HelpEntry::new(Action::StartSweep, "R", "Sweep cleanup"),
            HelpEntry::new(Action::CycleWorktreeSortMode, "s", "Cycle sort mode"),
            HelpEntry::new(Action::ShowBaseBranchPicker, "b", "Change base branch"),
            HelpEntry::new(Action::ShowProjectPicker, "p", "Switch project"),
            HelpEntry::new(Action::EnterFilterMode, "/", "Filter worktrees"),
            HelpEntry::new(Action::CycleColorScheme, "T", "Cycle theme"),
            HelpEntry::static_row("1-9", "Quick jump"),
        ],
        Context::DiffNormal => vec![
            HelpEntry::new(Action::ShowHelp, "?", "Show help"),
            HelpEntry::new(Action::CloseDiff, "q/Esc", "Close diff"),
            HelpEntry::new(Action::ScrollDown, "j/k", "Scroll line"),
            HelpEntry::new(Action::ScrollPageDown, "Ctrl+d/u", "Scroll page"),
            HelpEntry::new(Action::ToggleDiffType, "Tab", "Toggle WIP/Review"),
            HelpEntry::new(Action::EnterPatchMode, "a", "Enter patch mode (WIP only)"),
            HelpEntry::new(Action::SendCommitDiff, "c", "Commit changes"),
            HelpEntry::new(Action::TriggerMergeDiff, "m", "Merge branch"),
        ],
        Context::Patch => vec![
            HelpEntry::new(Action::ShowHelp, "?", "Show help"),
            HelpEntry::new(Action::StageAndNext, "y", "Stage hunk"),
            HelpEntry::new(Action::SkipHunk, "n", "Skip hunk"),
            HelpEntry::new(Action::UndoStagedHunk, "u", "Undo last staged"),
            HelpEntry::new(Action::SplitHunk, "s", "Split hunk"),
            HelpEntry::new(Action::StartComment, "o", "Add comment"),
            HelpEntry::new(Action::NextHunk, "j/k", "Next/prev hunk"),
            HelpEntry::new(Action::ScrollPageDown, "Ctrl+d/u", "Scroll hunk"),
            HelpEntry::new(Action::SendCommitDiff, "c", "Commit changes"),
            HelpEntry::new(Action::TriggerMergeDiff, "m", "Merge branch"),
            HelpEntry::new(Action::ExitPatchMode, "q/Esc", "Exit patch mode"),
        ],
        Context::Comment => vec![
            HelpEntry::static_row("Esc", "Cancel"),
            HelpEntry::static_row("Enter", "Send comment"),
            HelpEntry::static_row("<type>", "Input text"),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn default_keymap() -> Keymap {
        Keymap::default()
    }

    fn keymap_with(ctx: &str, bindings: Vec<(&str, Vec<&str>)>) -> Keymap {
        let map: BTreeMap<String, Vec<String>> = bindings
            .into_iter()
            .map(|(action, keys)| {
                (
                    action.to_string(),
                    keys.into_iter().map(|k| k.to_string()).collect(),
                )
            })
            .collect();

        let config = match ctx {
            "normal" => KeybindingsConfig {
                normal: Some(map),
                ..Default::default()
            },
            "worktree" => KeybindingsConfig {
                worktree: Some(map),
                ..Default::default()
            },
            "diff" => KeybindingsConfig {
                diff: Some(map),
                ..Default::default()
            },
            "patch" => KeybindingsConfig {
                patch: Some(map),
                ..Default::default()
            },
            _ => unreachable!(),
        };
        Keymap::new(&Some(config))
    }

    #[test]
    fn test_each_context_has_help_rows() {
        let km = default_keymap();
        assert!(!km.help_rows(Context::DashboardNormal).is_empty());
        assert!(!km.help_rows(Context::DashboardInput).is_empty());
        assert!(!km.help_rows(Context::DashboardFilter).is_empty());
        assert!(!km.help_rows(Context::WorktreeNormal).is_empty());
        assert!(!km.help_rows(Context::WorktreeFilter).is_empty());
        assert!(!km.help_rows(Context::DiffNormal).is_empty());
        assert!(!km.help_rows(Context::Patch).is_empty());
        assert!(!km.help_rows(Context::Comment).is_empty());
    }

    #[test]
    fn test_no_duplicate_keys_in_context() {
        let km = default_keymap();
        for ctx in [
            Context::DashboardNormal,
            Context::DashboardInput,
            Context::DashboardFilter,
            Context::WorktreeNormal,
            Context::WorktreeFilter,
            Context::DiffNormal,
            Context::Patch,
            Context::Comment,
        ] {
            let rows = km.help_rows(ctx);
            let keys: Vec<_> = rows.iter().map(|(k, _)| k.as_str()).collect();
            let mut seen = std::collections::HashSet::new();
            for key in &keys {
                assert!(
                    seen.insert(*key),
                    "Duplicate key '{key}' in context {ctx:?}"
                );
            }
        }
    }

    #[test]
    fn test_dashboard_quit_keys() {
        let km = default_keymap();
        let q = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        let esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);

        assert_eq!(km.resolve(Context::DashboardNormal, q), Some(Action::Quit));
        assert_eq!(
            km.resolve(Context::DashboardNormal, esc),
            Some(Action::Quit)
        );
        assert_eq!(
            km.resolve(Context::DashboardNormal, ctrl_c),
            Some(Action::Quit)
        );
    }

    #[test]
    fn test_diff_close_keys() {
        let km = default_keymap();
        let q = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        let esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);

        assert_eq!(km.resolve(Context::DiffNormal, q), Some(Action::CloseDiff));
        assert_eq!(
            km.resolve(Context::DiffNormal, esc),
            Some(Action::CloseDiff)
        );
    }

    #[test]
    fn test_patch_stage_key() {
        let km = default_keymap();
        let y = KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE);
        assert_eq!(km.resolve(Context::Patch, y), Some(Action::StageAndNext));
    }

    #[test]
    fn test_scope_filter_key() {
        let km = default_keymap();
        let shift_f = KeyEvent::new(KeyCode::Char('F'), KeyModifiers::NONE);
        assert_eq!(
            km.resolve(Context::DashboardNormal, shift_f),
            Some(Action::ToggleScopeFilter)
        );
    }

    // === Override tests ===

    #[test]
    fn test_override_removes_default_key() {
        // Configure quit with only "q" — ESC should no longer quit
        let km = keymap_with("normal", vec![("quit", vec!["q"])]);
        let q = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        let esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);

        assert_eq!(km.resolve(Context::DashboardNormal, q), Some(Action::Quit));
        assert_eq!(km.resolve(Context::DashboardNormal, esc), None);
    }

    #[test]
    fn test_override_adds_new_key() {
        // Configure quit with "q" and "x"
        let km = keymap_with("normal", vec![("quit", vec!["q", "x"])]);
        let x = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE);

        assert_eq!(km.resolve(Context::DashboardNormal, x), Some(Action::Quit));
    }

    #[test]
    fn test_non_overridden_actions_keep_defaults() {
        // Override quit, but next/previous should still work
        let km = keymap_with("normal", vec![("quit", vec!["q"])]);
        let j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        let k = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);

        assert_eq!(km.resolve(Context::DashboardNormal, j), Some(Action::Next));
        assert_eq!(
            km.resolve(Context::DashboardNormal, k),
            Some(Action::Previous)
        );
    }

    #[test]
    fn test_override_does_not_affect_other_contexts() {
        // Override quit in normal — diff should still have ESC for close
        let km = keymap_with("normal", vec![("quit", vec!["q"])]);
        let esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);

        assert_eq!(
            km.resolve(Context::DiffNormal, esc),
            Some(Action::CloseDiff)
        );
    }

    #[test]
    fn test_invalid_action_name_ignored() {
        // Unknown action should be silently skipped (with warning log)
        let km = keymap_with("normal", vec![("bogus_action", vec!["q"])]);
        let q = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);

        // q should still trigger default Quit since the override was invalid
        assert_eq!(km.resolve(Context::DashboardNormal, q), Some(Action::Quit));
    }

    #[test]
    fn test_invalid_key_string_ignored() {
        // Unknown key string should be silently skipped
        let km = keymap_with("normal", vec![("quit", vec!["invalid_key_name"])]);
        let q = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);

        // quit was overridden (action is in overridden set), but no valid keys were added
        // so "q" default is suppressed
        assert_eq!(km.resolve(Context::DashboardNormal, q), None);
    }

    #[test]
    fn test_parse_key_variants() {
        assert_eq!(
            parse_key("q"),
            Some(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE))
        );
        assert_eq!(
            parse_key("esc"),
            Some(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
        );
        assert_eq!(
            parse_key("Esc"),
            Some(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
        );
        assert_eq!(
            parse_key("enter"),
            Some(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
        );
        assert_eq!(
            parse_key("tab"),
            Some(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE))
        );
        assert_eq!(
            parse_key("ctrl+c"),
            Some(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL))
        );
        assert_eq!(
            parse_key("up"),
            Some(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE))
        );
        assert_eq!(
            parse_key("pagedown"),
            Some(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE))
        );
        assert_eq!(
            parse_key("+"),
            Some(KeyEvent::new(KeyCode::Char('+'), KeyModifiers::NONE))
        );
        assert_eq!(parse_key("invalid_long_string"), None);
    }

    #[test]
    fn test_help_rows_reflect_override() {
        let km = keymap_with("normal", vec![("quit", vec!["q", "ctrl+c"])]);
        let rows = km.help_rows(Context::DashboardNormal);
        let quit_row = rows.iter().find(|(_, desc)| desc == "Quit");
        assert!(quit_row.is_some());
        let (keys, _) = quit_row.unwrap();
        assert!(!keys.contains("Esc"));
        assert!(keys.contains('q'));
    }
}
