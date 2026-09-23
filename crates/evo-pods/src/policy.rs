//! Instance policies: how a pod launches an *isolated* copy of an app.
//!
//! The pod crate itself ships **zero** application knowledge
//! (`default_policies()` is empty): launching an isolated VS Code or
//! Chromium instance requires per-product flags (`--user-data-dir`), and
//! encoding those in code is exactly the hardcoding this system forbids.
//! Instead the host reads a policy file — user state, not code — of
//! lines:
//!
//! ```text
//! # evo-instance-policies-v1
//! app=Visual Studio Code|data_dir_flag=--user-data-dir=|extra_flag=--extensions-dir=
//! app=Google Chrome|data_dir_flag=--user-data-dir=
//! ```
//!
//! matched by the app's witnessed name. Integrators provide the files; Evo
//! never presumes products.

use std::collections::BTreeMap;

pub const POLICY_HEADER: &str = "# evo-instance-policies-v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstancePolicy {
    /// Witnessed application name this policy applies to.
    pub app: String,
    /// Flag prefix; launch passes `{prefix}{pod_profile_dir}`.
    pub data_dir_flag: String,
    /// Optional secondary isolation flag (same substitution).
    pub extra_flag: Option<String>,
}

impl InstancePolicy {
    /// The args for one pod launch, given the pod's absolute profile dir.
    pub fn launch_args(&self, pod_profile_dir: &str) -> Vec<String> {
        let mut args = vec![format!("{}{}", self.data_dir_flag, pod_profile_dir)];
        if let Some(extra) = &self.extra_flag {
            args.push(format!("{}{}", extra, pod_profile_dir));
        }
        args
    }
}

pub fn parse_policies(text: &str) -> Result<Vec<InstancePolicy>, String> {
    let mut policies = Vec::new();
    for (lineno, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut fields: BTreeMap<&str, &str> = BTreeMap::new();
        for pair in line.split('|') {
            let Some((key, value)) = pair.split_once('=') else {
                return Err(format!("line {}: expected key=value, got {pair:?}", lineno + 1));
            };
            fields.insert(key.trim(), value.trim());
        }
        let app = fields
            .remove("app")
            .filter(|s| !s.is_empty())
            .ok_or_else(|| format!("line {}: missing app=", lineno + 1))?
            .to_string();
        let data_dir_flag = fields
            .remove("data_dir_flag")
            .filter(|s| !s.is_empty())
            .ok_or_else(|| format!("line {}: missing data_dir_flag=", lineno + 1))?
            .to_string();
        let extra_flag = fields.remove("extra_flag").filter(|s| !s.is_empty()).map(str::to_string);
        policies.push(InstancePolicy { app, data_dir_flag, extra_flag });
    }
    Ok(policies)
}

pub fn format_policies(policies: &[InstancePolicy]) -> String {
    let mut out = String::from(POLICY_HEADER);
    out.push('\n');
    for p in policies {
        out.push_str(&format!("app={}|data_dir_flag={}", p.app, p.data_dir_flag));
        if let Some(extra) = &p.extra_flag {
            out.push_str(&format!("|extra_flag={extra}"));
        }
        out.push('\n');
    }
    out
}

/// The shipped set is empty. Product-specific knowledge enters only via
/// files the person (or an integrator) declares — see IS-0023 §2.1.
pub fn default_policies() -> Vec<InstancePolicy> {
    Vec::new()
}

/// Policy lookup by witnessed app name (exact match — the capture layer's
/// own string, no fuzzy guessing).
pub fn policy_for<'a>(policies: &'a [InstancePolicy], app_name: &str) -> Option<&'a InstancePolicy> {
    policies.iter().find(|p| p.app == app_name)
}
