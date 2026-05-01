use std::path::Path;

/// Path prefixes (relative to home) whose contents are always write-protected.
///
/// Matches any file whose canonical path contains one of these components, preventing
/// accidental or injected overwrites of credentials, keys, and shell init files.
const SENSITIVE_HOME_PREFIXES: &[&str] = &[
    ".ssh",
    ".aws",
    ".gnupg",
    ".kube",
    ".docker", // docker config / credentials
    ".npmrc",  // npm auth tokens
    ".pypirc", // PyPI credentials
    ".netrc",  // generic credentials
    ".password-store",
];

/// Exact filenames (anywhere in the path) that are always write-protected.
const SENSITIVE_FILENAMES: &[&str] = &[
    ".bashrc",
    ".zshrc",
    ".profile",
    ".bash_profile",
    ".bash_history",
    ".zsh_history",
    ".fish_history",
    ".gitconfig", // may contain tokens (e.g. HTTPS helpers)
    "authorized_keys",
    "known_hosts",
    "id_rsa",
    "id_ed25519",
    "id_ecdsa",
    "id_dsa",
];

/// Absolute path prefixes that are always write-protected (system files).
const SENSITIVE_ABSOLUTE_PREFIXES: &[&str] = &[
    "/etc/passwd",
    "/etc/shadow",
    "/etc/sudoers",
    "/etc/crontab",
    "/etc/cron.",
    "/etc/ssh/",
];

/// Returns `true` if writing to `path` should be unconditionally blocked.
///
/// Checks the path string representation (not resolved canonical path) so it
/// works for files that don't exist yet. Call after the allowed-roots check;
/// this is an additional deny list for sensitive locations within allowed roots.
pub fn is_sensitive_write_path(path: &Path) -> bool {
    let path_str = path.to_string_lossy().to_lowercase();

    // Absolute prefixes
    for prefix in SENSITIVE_ABSOLUTE_PREFIXES {
        if path_str.starts_with(prefix) {
            return true;
        }
    }

    // Home-relative prefixes: match /<any>/.ssh/ or .ssh/ etc.
    for prefix in SENSITIVE_HOME_PREFIXES {
        // Matches as a path component anywhere in the string
        let slash_prefix = format!("/{prefix}");
        if path_str.contains(&slash_prefix) || path_str.starts_with(prefix) {
            return true;
        }
    }

    // Sensitive filenames (final component)
    if let Some(name) = path.file_name() {
        let name_lower = name.to_string_lossy().to_lowercase();
        for fname in SENSITIVE_FILENAMES {
            if name_lower == *fname {
                return true;
            }
        }
    }

    false
}

/// Returns `true` if `cmd` contains a string reference to any sensitive path.
///
/// Used as a terminal hardline check. String-based matching is inherently
/// incomplete (shell variables, subshells can evade it); the Docker sandbox is
/// the primary control for terminal isolation.
pub fn command_references_sensitive_path(cmd: &str) -> bool {
    let lower = cmd.to_lowercase();

    for prefix in SENSITIVE_HOME_PREFIXES {
        let dotted = format!("/.{prefix}/");
        let slash = format!("/{prefix}/");
        if lower.contains(&dotted) || lower.contains(&slash) {
            return true;
        }
        // e.g. ~/.ssh
        let tilde = format!("~/{prefix}");
        if lower.contains(&tilde) {
            return true;
        }
    }

    for fname in SENSITIVE_FILENAMES {
        if lower.contains(fname) {
            return true;
        }
    }

    for prefix in SENSITIVE_ABSOLUTE_PREFIXES {
        if lower.contains(prefix) {
            return true;
        }
    }

    false
}

/// Replace each secret value in `output` with `[REDACTED]`.
///
/// Skips values shorter than 8 characters to avoid false positives on common
/// short strings. Empty or whitespace-only secrets are also skipped.
pub fn redact_secrets(mut output: String, secrets: &[&str]) -> String {
    for secret in secrets {
        let secret = secret.trim();
        if secret.len() < 8 {
            continue;
        }
        if output.contains(secret) {
            output = output.replace(secret, "[REDACTED]");
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // ── is_sensitive_write_path ───────────────────────────────────────────────

    #[test]
    fn blocks_ssh_directory() {
        assert!(is_sensitive_write_path(Path::new(
            "/home/user/.ssh/authorized_keys"
        )));
        assert!(is_sensitive_write_path(Path::new("~/.ssh/config")));
    }

    #[test]
    fn blocks_aws_credentials() {
        assert!(is_sensitive_write_path(Path::new(
            "/home/user/.aws/credentials"
        )));
    }

    #[test]
    fn blocks_kube_config() {
        assert!(is_sensitive_write_path(Path::new(
            "/home/user/.kube/config"
        )));
    }

    #[test]
    fn blocks_shell_dotfiles() {
        for name in &[".bashrc", ".zshrc", ".profile", ".bash_profile"] {
            assert!(
                is_sensitive_write_path(&PathBuf::from(format!("/home/user/{name}"))),
                "{name} should be blocked"
            );
        }
    }

    #[test]
    fn blocks_private_keys() {
        assert!(is_sensitive_write_path(Path::new("/home/user/.ssh/id_rsa")));
        assert!(is_sensitive_write_path(Path::new("/tmp/id_ed25519")));
    }

    #[test]
    fn blocks_system_files() {
        assert!(is_sensitive_write_path(Path::new("/etc/passwd")));
        assert!(is_sensitive_write_path(Path::new("/etc/shadow")));
        assert!(is_sensitive_write_path(Path::new("/etc/sudoers")));
    }

    #[test]
    fn allows_normal_paths() {
        assert!(!is_sensitive_write_path(Path::new(
            "/home/user/project/main.rs"
        )));
        assert!(!is_sensitive_write_path(Path::new("/tmp/output.txt")));
        assert!(!is_sensitive_write_path(Path::new("./README.md")));
    }

    // ── command_references_sensitive_path ─────────────────────────────────────

    #[test]
    fn detects_ssh_reference_in_command() {
        assert!(command_references_sensitive_path(
            "cat > ~/.ssh/authorized_keys"
        ));
        assert!(command_references_sensitive_path(
            "echo key >> /home/user/.ssh/authorized_keys"
        ));
    }

    #[test]
    fn detects_bashrc_reference() {
        assert!(command_references_sensitive_path("echo alias >> ~/.bashrc"));
    }

    #[test]
    fn allows_clean_commands() {
        assert!(!command_references_sensitive_path("ls -la /tmp"));
        assert!(!command_references_sensitive_path("cargo build --release"));
    }

    // ── redact_secrets ────────────────────────────────────────────────────────

    #[test]
    fn redacts_known_secret() {
        let output = "error: invalid key sk-ant-api03-verylongapikey123".to_string();
        let result = redact_secrets(output, &["sk-ant-api03-verylongapikey123"]);
        assert!(result.contains("[REDACTED]"));
        assert!(!result.contains("sk-ant-api03"));
    }

    #[test]
    fn skips_short_secrets() {
        let output = "user: abc".to_string();
        let result = redact_secrets(output.clone(), &["abc"]);
        assert_eq!(result, output); // too short, not redacted
    }

    #[test]
    fn redacts_multiple_secrets() {
        let output = "key1=longapikey1here key2=longapikey2here".to_string();
        let result = redact_secrets(output, &["longapikey1here", "longapikey2here"]);
        assert_eq!(result, "key1=[REDACTED] key2=[REDACTED]");
    }

    #[test]
    fn handles_empty_secrets_list() {
        let output = "normal output".to_string();
        let result = redact_secrets(output.clone(), &[]);
        assert_eq!(result, output);
    }
}
