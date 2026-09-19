//! The `/terminal-setup` command: make Shift+Enter actually work.
//!
//! See [`crate::tui::terminal_setup`] for why terminals cannot report
//! Shift+Enter by default. This module is the user-facing half: it reports
//! whether the current terminal can report the chord, and applies the fix when
//! configuration can provide one.

use jcode_tui_messages::DisplayMessage;

use super::App;
use crate::tui::terminal_setup::{self, Applied};

impl App {
    pub(super) fn handle_terminal_setup_command(&mut self, trimmed: &str) -> bool {
        if trimmed != "/terminal-setup" {
            return false;
        }

        let inside_tmux = std::env::var_os("TMUX").is_some();

        // If the terminal already reports modified Enter, Shift+Enter works and
        // changing keyboard config would be noise. Inside tmux we still run
        // setup: the same config file also enables graphics passthrough, which
        // tmux masks regardless of whether the keyboard protocol works.
        if !inside_tmux && terminal_setup::supports_modified_enter_reporting() == Some(true) {
            self.push_display_message(DisplayMessage::system(
                "✓ Shift+Enter already works: this terminal reports modified Enter \
                 via the kitty keyboard protocol.\n\
                 Press Shift+Enter to insert a newline; Enter submits."
                    .to_string(),
            ));
            return true;
        }

        let term_program = std::env::var("TERM_PROGRAM").ok();
        let target = terminal_setup::diagnose(term_program.as_deref(), inside_tmux);

        let home = match dirs::home_dir() {
            Some(home) => home,
            None => {
                self.push_display_message(DisplayMessage::error(
                    "Cannot locate your home directory, so terminal config cannot be updated."
                        .to_string(),
                ));
                return true;
            }
        };

        match terminal_setup::apply(target, &home) {
            Ok(Applied::Changed { detail }) => {
                let what = match target {
                    terminal_setup::SetupTarget::Tmux => {
                        "Shift+Enter and inline images (passthrough)"
                    }
                    _ => "Shift+Enter",
                };
                self.push_display_message(DisplayMessage::system(format!(
                    "✓ Configured {} for {}.\n{detail}\n{}",
                    target.label(),
                    what,
                    target.activation_note()
                )));
            }
            Ok(Applied::AlreadyConfigured) => {
                let what = match target {
                    terminal_setup::SetupTarget::Tmux => {
                        "Shift+Enter and inline images (passthrough)"
                    }
                    _ => "Shift+Enter",
                };
                self.push_display_message(DisplayMessage::system(format!(
                    "{} is already configured for {}.\nIf it still does not work, {}",
                    target.label(),
                    what,
                    target.activation_note()
                )));
            }
            Ok(Applied::Manual { message }) => {
                self.push_display_message(DisplayMessage::system(message));
            }
            Err(err) => {
                self.push_display_message(DisplayMessage::error(format!(
                    "Failed to update {} configuration: {err}",
                    target.label()
                )));
            }
        }
        true
    }
}
