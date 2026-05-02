use async_trait::async_trait;
use garudust_core::{
    config::TerminalSandbox,
    error::ToolError,
    tool::{Tool, ToolContext},
    types::ToolResult,
};
use serde_json::json;
use tokio::process::Command;

use crate::security::{command_references_sensitive_path, redact_secrets};

/// Only these variables are forwarded to subprocesses.
/// Secrets (API keys, tokens, passwords) are deliberately excluded.
const ENV_ALLOWLIST: &[&str] = &[
    "PATH", "HOME", "USER", "LOGNAME", "SHELL", "LANG", "LC_ALL", "TMPDIR", "TEMP", "TMP", "TERM",
];

/// Maximum combined stdout+stderr returned to the model (bytes).
/// Prevents context bloat from runaway commands. Truncated as 40% head + 60% tail.
const MAX_OUTPUT_BYTES: usize = 51_200; // 50 KB

pub struct Terminal;

// ── Hardline safety checks ────────────────────────────────────────────────────

/// Check `cmd` against unconditionally-blocked patterns.
///
/// These are blocked regardless of approval mode or sandbox configuration.
/// The list covers the most destructive single-command attacks; defence-in-depth
/// is provided by the sandbox, approval gate, and env filtering layers.
pub fn check_hardline(cmd: &str) -> Result<(), ToolError> {
    let lower = cmd.to_lowercase();

    if is_recursive_root_deletion(&lower) {
        return Err(ToolError::Execution(
            "blocked: recursive root filesystem deletion".into(),
        ));
    }

    // mkfs — formats a filesystem (any variant: mkfs.ext4, mkfs.vfat, …)
    if any_segment_starts_with(&lower, &["mkfs"]) {
        return Err(ToolError::Execution(
            "blocked: filesystem format command (mkfs)".into(),
        ));
    }

    // Fork bomb — classic :(){ :|:& };: and common whitespace variants
    if lower.contains(":()")
        && lower.contains(":|:")
        && (lower.contains("};:") || lower.contains("}; :"))
    {
        return Err(ToolError::Execution("blocked: fork bomb pattern".into()));
    }

    // dd writing to a raw block device (of=/dev/sd*, /dev/hd*, /dev/nvme*, /dev/vd*)
    if lower.contains("dd") && contains_block_device_write(&lower) {
        return Err(ToolError::Execution(
            "blocked: dd writing to raw block device".into(),
        ));
    }

    // Redirecting directly to a block device (> /dev/sda style)
    if contains_redirect_to_block_device(&lower) {
        return Err(ToolError::Execution(
            "blocked: shell redirect to raw block device".into(),
        ));
    }

    // System shutdown / reboot
    if any_segment_starts_with(
        &lower,
        &["shutdown", "reboot", "halt", "poweroff", "init 0", "init 6"],
    ) {
        return Err(ToolError::Execution(
            "blocked: system shutdown or reboot command".into(),
        ));
    }

    // systemctl power-management subcommands
    if lower.contains("systemctl")
        && (lower.contains("poweroff")
            || lower.contains("reboot")
            || lower.contains("halt")
            || lower.contains("kexec"))
    {
        return Err(ToolError::Execution(
            "blocked: systemctl power-management command".into(),
        ));
    }

    // Sensitive credential / system file references
    if command_references_sensitive_path(cmd) {
        return Err(ToolError::Execution(
            "blocked: command references a protected credential or system file".into(),
        ));
    }

    Ok(())
}

/// Returns `true` if `cmd` (lowercase) contains `rm` with a recursive flag
/// targeting `/` or `/*`.
fn is_recursive_root_deletion(cmd: &str) -> bool {
    // Tokenise every pipeline/sequence segment and check each independently.
    for seg in split_shell_segments(cmd) {
        let tokens: Vec<&str> = seg.split_whitespace().collect();

        // Must have rm (or a path-prefixed rm like /bin/rm)
        let has_rm = tokens.iter().any(|t| *t == "rm" || t.ends_with("/rm"));
        if !has_rm {
            continue;
        }

        // Must have a recursive flag
        // Combined short flags containing r: -rf, -fr, -Rf, -fR, -rR …
        let has_recursive = tokens.iter().any(|t| {
            *t == "-r"
                || *t == "--recursive"
                || (t.starts_with('-') && !t.starts_with("--") && t.contains('r'))
        });
        if !has_recursive {
            continue;
        }

        // Must target root path
        let targets_root = tokens.iter().any(|t| *t == "/" || *t == "/*" || *t == "/.");
        if targets_root {
            return true;
        }
    }
    false
}

/// Returns `true` if the (lowercase) command contains `of=/dev/<block-device>`.
fn contains_block_device_write(cmd: &str) -> bool {
    for prefix in &[
        "of=/dev/sd",
        "of=/dev/hd",
        "of=/dev/nvme",
        "of=/dev/vd",
        "of=/dev/xvd",
        "of=/dev/mmcblk",
    ] {
        if cmd.contains(prefix) {
            return true;
        }
    }
    false
}

/// Returns `true` if the (lowercase) command redirects stdout to a block device.
fn contains_redirect_to_block_device(cmd: &str) -> bool {
    for prefix in &[
        ">/dev/sd",
        "> /dev/sd",
        ">/dev/hd",
        "> /dev/hd",
        ">/dev/nvme",
        "> /dev/nvme",
    ] {
        if cmd.contains(prefix) {
            return true;
        }
    }
    false
}

/// Split `cmd` on common shell sequence operators (`; | & \n`) and return the
/// first token of each resulting segment.
fn any_segment_starts_with(cmd: &str, prefixes: &[&str]) -> bool {
    for seg in split_shell_segments(cmd) {
        let first = seg.split_whitespace().next().unwrap_or("");
        // Strip leading path component (/sbin/shutdown → shutdown)
        let name = first.rsplit('/').next().unwrap_or(first);
        if prefixes
            .iter()
            .any(|p| name == *p || seg.trim_start().starts_with(p))
        {
            return true;
        }
    }
    false
}

/// Split a shell command string into individual command segments at `;`, `|`, `&`, `\n`.
fn split_shell_segments(cmd: &str) -> impl Iterator<Item = &str> {
    cmd.split([';', '|', '&', '\n'])
}

// ── Output helpers ────────────────────────────────────────────────────────────

/// Truncate output larger than `MAX_OUTPUT_BYTES` as 40% head + 60% tail with
/// an omission notice, matching Hermes's approach.
fn truncate_output(s: String) -> String {
    if s.len() <= MAX_OUTPUT_BYTES {
        return s;
    }
    let head_len = MAX_OUTPUT_BYTES * 2 / 5;
    let tail_len = MAX_OUTPUT_BYTES - head_len;
    let head = &s[..head_len];
    let tail = &s[s.len() - tail_len..];
    let omitted = s.len() - head_len - tail_len;
    format!("{head}\n\n[... {omitted} bytes omitted ...]\n\n{tail}")
}

// ── Secret collection for output redaction ────────────────────────────────────

/// Collect known secret values from config and environment for output redaction.
fn collect_secrets(ctx: &ToolContext) -> Vec<String> {
    let mut secrets = Vec::new();
    if let Some(k) = &ctx.config.api_key {
        secrets.push(k.clone());
    }
    if let Some(k) = &ctx.config.security.gateway_api_key {
        secrets.push(k.clone());
    }
    // Also collect from known secret env var names in case they slipped through
    for var in &[
        "ANTHROPIC_API_KEY",
        "OPENROUTER_API_KEY",
        "OPENAI_API_KEY",
        "GARUDUST_API_KEY",
    ] {
        if let Ok(v) = std::env::var(var) {
            secrets.push(v);
        }
    }
    secrets
}

// ── Docker command builder ────────────────────────────────────────────────────

/// Return the `docker run` argument list for `shell_cmd`.
/// Separated from `build_docker_command` so tests can inspect args without
/// spawning a process.
fn docker_run_args(shell_cmd: &str, ctx: &ToolContext) -> Vec<String> {
    let security = &ctx.config.security;
    let mut args: Vec<String> = vec![
        "run".into(),
        "--rm".into(),
        "--cap-drop".into(),
        "ALL".into(),
        "--security-opt".into(),
        "no-new-privileges:true".into(),
        "--pids-limit".into(),
        "256".into(),
        "--tmpfs".into(),
        "/tmp:size=512m".into(),
    ];

    if let Ok(cwd) = std::env::current_dir() {
        args.push("-v".into());
        args.push(format!("{}:/workspace:rw", cwd.display()));
        args.push("-w".into());
        args.push("/workspace".into());
    }

    for opt in &security.terminal_sandbox_opts {
        for part in opt.split_whitespace() {
            args.push(part.into());
        }
    }

    args.push(security.terminal_sandbox_image.clone());
    args.extend(["sh".into(), "-c".into(), shell_cmd.into()]);
    args
}

/// Build a `docker run` command that wraps `shell_cmd` with hardened defaults.
fn build_docker_command(shell_cmd: &str, ctx: &ToolContext) -> Command {
    let mut cmd = Command::new("docker");
    cmd.args(docker_run_args(shell_cmd, ctx));

    // Env-clear the docker process itself (secrets never reach the container via env)
    cmd.env_clear();
    for key in ENV_ALLOWLIST {
        if let Ok(val) = std::env::var(key) {
            cmd.env(key, val);
        }
    }

    cmd
}

// ── Tool impl ─────────────────────────────────────────────────────────────────

#[async_trait]
impl Tool for Terminal {
    fn name(&self) -> &'static str {
        "terminal"
    }
    fn description(&self) -> &'static str {
        "Run a shell command and return the output"
    }
    fn toolset(&self) -> &'static str {
        "terminal"
    }

    fn is_destructive(&self) -> bool {
        true
    }

    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "command":     { "type": "string", "description": "Shell command to execute" },
                "description": { "type": "string", "description": "What this command does" },
                "timeout_secs": { "type": "integer", "default": 30 }
            },
            "required": ["command", "description"]
        })
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        ctx: &ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let command = params["command"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("command required".into()))?;
        let timeout_secs = params["timeout_secs"].as_u64().unwrap_or(30);

        // Layer 1: unconditional hardline blocks (includes sensitive path check)
        check_hardline(command)?;

        // Approval and audit logging are handled by ToolRegistry::dispatch()
        // via the is_destructive() property — no per-tool check needed here.

        // Layer 2: sandbox or local execution
        let mut cmd = match ctx.config.security.terminal_sandbox {
            TerminalSandbox::Docker => build_docker_command(command, ctx),
            TerminalSandbox::None => {
                let mut c = Command::new("sh");
                c.arg("-c").arg(command);
                c.env_clear();
                for key in ENV_ALLOWLIST {
                    if let Ok(val) = std::env::var(key) {
                        c.env(key, val);
                    }
                }
                c
            }
        };

        let output =
            tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), cmd.output())
                .await
                .map_err(|_| ToolError::Timeout(timeout_secs))?
                .map_err(|e| {
                    // ENOENT on the docker binary means Docker is not installed.
                    if ctx.config.security.terminal_sandbox == TerminalSandbox::Docker
                        && e.kind() == std::io::ErrorKind::NotFound
                    {
                        ToolError::Execution(
                            "Docker is not installed or not in PATH. \
                             Set `terminal_sandbox: none` in config or install Docker."
                                .into(),
                        )
                    } else {
                        ToolError::Execution(e.to_string())
                    }
                })?;

        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

        let combined = if stderr.is_empty() {
            stdout
        } else if stdout.is_empty() {
            stderr
        } else {
            format!("{stdout}\n[stderr]\n{stderr}")
        };

        // Layer 3: output hardening — truncate then redact secrets
        let combined = truncate_output(combined);
        let secret_values = collect_secrets(ctx);
        let secret_refs: Vec<&str> = secret_values.iter().map(String::as_str).collect();
        let combined = redact_secrets(combined, &secret_refs);

        let is_error = !output.status.success();
        if is_error {
            Ok(ToolResult::err("", combined))
        } else {
            Ok(ToolResult::ok("", combined))
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use garudust_core::{
        budget::IterationBudget,
        config::{AgentConfig, SecurityConfig, TerminalSandbox},
        error::AgentError,
        memory::{MemoryContent, MemoryStore},
        tool::{ApprovalDecision, CommandApprover, ToolContext},
    };

    use super::*;

    // ── Test helpers ──────────────────────────────────────────────────────────

    struct NoopMemory;

    #[async_trait]
    impl MemoryStore for NoopMemory {
        async fn read_memory(&self) -> Result<MemoryContent, AgentError> {
            Ok(MemoryContent::default())
        }
        async fn write_memory(&self, _: &MemoryContent) -> Result<(), AgentError> {
            Ok(())
        }
        async fn read_user_profile(&self) -> Result<String, AgentError> {
            Ok(String::new())
        }
        async fn write_user_profile(&self, _: &str) -> Result<(), AgentError> {
            Ok(())
        }
    }

    struct AutoApprove;

    #[async_trait]
    impl CommandApprover for AutoApprove {
        async fn approve(&self, _: &str, _: &str) -> ApprovalDecision {
            ApprovalDecision::Approved
        }
    }

    fn make_ctx(sandbox: TerminalSandbox) -> ToolContext {
        let config = AgentConfig {
            security: SecurityConfig {
                terminal_sandbox: sandbox,
                terminal_sandbox_image: "ubuntu:24.04".into(),
                terminal_sandbox_opts: vec![],
                ..Default::default()
            },
            ..AgentConfig::default()
        };
        ToolContext {
            session_id: "test".into(),
            agent_id: "test".into(),
            iteration: 0,
            budget: Arc::new(IterationBudget::new(10)),
            memory: Arc::new(NoopMemory),
            config: Arc::new(config),
            approver: Arc::new(AutoApprove),
            sub_agent: None,
        }
    }

    // ── docker_run_args ───────────────────────────────────────────────────────

    #[test]
    fn docker_args_contain_hardened_flags() {
        let ctx = make_ctx(TerminalSandbox::Docker);
        let args = docker_run_args("echo hello", &ctx);

        assert_eq!(args[0], "run");
        assert_eq!(args[1], "--rm");
        assert!(args.contains(&"--cap-drop".into()), "missing --cap-drop");
        assert!(args.contains(&"ALL".into()), "missing ALL after --cap-drop");
        assert!(
            args.contains(&"--security-opt".into()),
            "missing --security-opt"
        );
        assert!(
            args.contains(&"no-new-privileges:true".into()),
            "missing no-new-privileges:true"
        );
        assert!(args.contains(&"--pids-limit".into()));
        assert!(args.contains(&"256".into()));
        assert!(args.contains(&"--tmpfs".into()));
        assert!(args.iter().any(|a| a.contains("/tmp:size=512m")));
    }

    #[test]
    fn docker_args_mount_cwd_as_workspace() {
        let ctx = make_ctx(TerminalSandbox::Docker);
        let args = docker_run_args("ls", &ctx);

        let has_workspace_mount = args.iter().any(|a| a.contains(":/workspace:rw"));
        assert!(has_workspace_mount, "expected cwd:/workspace:rw mount");
        assert!(args.contains(&"-w".into()));
        assert!(args.contains(&"/workspace".into()));
    }

    #[test]
    fn docker_args_end_with_image_and_command() {
        let ctx = make_ctx(TerminalSandbox::Docker);
        let args = docker_run_args("echo hello", &ctx);

        // Last 4: [..., "ubuntu:24.04", "sh", "-c", "echo hello"]
        let n = args.len();
        assert!(n >= 4);
        assert_eq!(args[n - 4], "ubuntu:24.04");
        assert_eq!(args[n - 3], "sh");
        assert_eq!(args[n - 2], "-c");
        assert_eq!(args[n - 1], "echo hello");
    }

    #[test]
    fn docker_args_include_extra_opts() {
        let mut ctx = make_ctx(TerminalSandbox::Docker);
        Arc::get_mut(&mut ctx.config)
            .unwrap()
            .security
            .terminal_sandbox_opts = vec!["--network=none".into(), "--memory=512m".into()];

        let args = docker_run_args("ls", &ctx);
        assert!(args.contains(&"--network=none".into()));
        assert!(args.contains(&"--memory=512m".into()));
    }

    // ── execute() sandbox routing ─────────────────────────────────────────────

    #[tokio::test]
    async fn no_sandbox_runs_locally() {
        let ctx = make_ctx(TerminalSandbox::None);
        let params = serde_json::json!({
            "command": "echo sandbox_test_marker",
            "description": "test"
        });
        let result = Terminal.execute(params, &ctx).await.unwrap();
        assert!(result.content.contains("sandbox_test_marker"));
    }

    #[tokio::test]
    async fn docker_sandbox_fails_without_docker_or_runs_in_container() {
        let ctx = make_ctx(TerminalSandbox::Docker);
        let params = serde_json::json!({
            "command": "echo docker_test_marker",
            "description": "test"
        });
        match Terminal.execute(params, &ctx).await {
            Ok(result) => {
                // Docker is available — command ran in a container
                assert!(
                    result.content.contains("docker_test_marker"),
                    "unexpected output: {}",
                    result.content
                );
            }
            Err(ToolError::Execution(msg)) => {
                // Docker is not installed — correct ENOENT error path
                assert!(
                    msg.contains("Docker is not installed"),
                    "unexpected error: {msg}"
                );
            }
            Err(e) => panic!("unexpected error variant: {e}"),
        }
    }

    // helpers
    fn ok(cmd: &str) {
        assert!(
            check_hardline(cmd).is_ok(),
            "expected ok but was blocked: {cmd:?}"
        );
    }
    fn blocked(cmd: &str) {
        assert!(
            check_hardline(cmd).is_err(),
            "expected blocked but was allowed: {cmd:?}"
        );
    }

    // ── recursive root deletion ───────────────────────────────────────────────

    #[test]
    fn blocks_rm_rf_root() {
        blocked("rm -rf /");
        blocked("rm -rf /  ");
        blocked("rm -fr /");
        blocked("rm -r -f /");
        blocked("rm --recursive -f /");
        blocked("rm -rf /*");
    }

    #[test]
    fn allows_rm_rf_subdir() {
        ok("rm -rf ./build");
        ok("rm -rf /tmp/myproject");
        ok("rm -rf ../dist");
        // Targeting a regular file — passes hardline, routed to approval gate via is_destructive()
        ok("rm -rf test.txt");
        ok("rm -rf some/nested/path/file.log");
    }

    #[test]
    fn blocks_rm_rf_root_in_pipeline() {
        blocked("echo done && rm -rf /");
        blocked("ls; rm -rf /*");
    }

    // ── mkfs ─────────────────────────────────────────────────────────────────

    #[test]
    fn blocks_mkfs_variants() {
        blocked("mkfs /dev/sda1");
        blocked("mkfs.ext4 /dev/sda1");
        blocked("mkfs.vfat /dev/sdb");
    }

    #[test]
    fn allows_strings_containing_mkfs_as_arg() {
        // "mkfs" only blocked as a leading command token
        ok("echo mkfs.ext4");
        ok("grep mkfs /etc/fstab");
    }

    // ── fork bomb ────────────────────────────────────────────────────────────

    #[test]
    fn blocks_fork_bomb() {
        blocked(":(){:|:&};:");
        blocked(":(){ :|:& };:");
    }

    // ── dd block device ──────────────────────────────────────────────────────

    #[test]
    fn blocks_dd_to_block_device() {
        blocked("dd if=/dev/zero of=/dev/sda");
        blocked("dd if=/dev/urandom of=/dev/nvme0n1");
        blocked("dd if=disk.img of=/dev/mmcblk0");
    }

    #[test]
    fn allows_dd_to_file() {
        ok("dd if=/dev/zero of=/tmp/test.img bs=1M count=10");
        ok("dd if=input.img of=output.img");
    }

    // ── redirect to block device ─────────────────────────────────────────────

    #[test]
    fn blocks_redirect_to_block_device() {
        blocked("cat /dev/zero > /dev/sda");
        blocked("echo foo >/dev/sda");
    }

    // ── shutdown / reboot ────────────────────────────────────────────────────

    #[test]
    fn blocks_shutdown_variants() {
        blocked("shutdown -h now");
        blocked("reboot");
        blocked("halt");
        blocked("poweroff");
        blocked("/sbin/reboot");
    }

    #[test]
    fn blocks_systemctl_power() {
        blocked("systemctl poweroff");
        blocked("systemctl reboot");
        blocked("systemctl halt");
    }

    #[test]
    fn allows_systemctl_other() {
        ok("systemctl status nginx");
        ok("systemctl restart nginx");
    }

    // ── sensitive path references ────────────────────────────────────────────

    #[test]
    fn blocks_ssh_key_write() {
        blocked("echo key > ~/.ssh/authorized_keys");
        blocked("cp mykey.pub /home/user/.ssh/authorized_keys");
    }

    #[test]
    fn blocks_bashrc_write() {
        blocked("echo alias ll=ls >> ~/.bashrc");
    }

    #[test]
    fn blocks_aws_credentials_write() {
        blocked("echo '[default]' > ~/.aws/credentials");
    }

    #[test]
    fn allows_read_of_home_directory() {
        ok("ls ~");
        ok("cat ~/README.md");
    }

    // ── output truncation ────────────────────────────────────────────────────

    #[test]
    fn truncate_output_short_passthrough() {
        let s = "hello world".to_string();
        assert_eq!(truncate_output(s.clone()), s);
    }

    #[test]
    fn truncate_output_long_contains_notice() {
        let s = "x".repeat(MAX_OUTPUT_BYTES + 1000);
        let result = truncate_output(s);
        assert!(result.contains("bytes omitted"));
        assert!(result.len() < MAX_OUTPUT_BYTES + 200);
    }
}
