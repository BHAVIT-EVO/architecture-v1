//! Exact, local macOS browser identity through an explicitly configured
//! browser DevTools endpoint.
//!
//! This module is intentionally a separate seam from [`browser_identity`].
//! `SurfaceIdentity` predates browser session evidence and cannot carry a
//! session token.  The contract here therefore keeps all browser identity
//! fields together:
//!
//! * the process id and the real `NSRunningApplication.launchDate` token;
//! * the browser-native window id returned by `Browser.getWindowForTarget`;
//! * the persistent profile token from the browser process arguments;
//! * the live DevTools browser-context token used as the session token;
//! * the exact DevTools target id and its exact URL.
//!
//! The concrete provider never starts a browser, discovers an endpoint, opens
//! a URL, or connects anywhere except the explicitly configured loopback
//! address.  DevTools target ids are retained as handles and are re-read by
//! target id, never by URL, title, array index, or recency.  If any required
//! identity is unavailable or ambiguous, the operation refuses.
//!
//! Chromium's default disk profile is NOT exposed by this protocol. Tabs
//! without `browserContextId` are unsupported; neither a process command line
//! nor a filesystem path can establish which disk profile owns a particular
//! tab. This provider supports only explicitly exposed browser contexts, not
//! persistent disk-profile identity when the browser exposes both the
//! `--user-data-dir` and `--profile-directory` process arguments, Safari, or
//! generic Accessibility tabs.
//! `Browser.getWindowForTarget` supplies a browser-native window id, NOT an
//! `AXIdentifier`/CGWindowID; callers must not relabel it as either. The two
//! namespaces cannot be mixed with existing `SurfaceIdentity` window tokens.

use crate::restore::{DispatchResult, VerificationResult};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Write};
use std::net::{IpAddr, Shutdown, SocketAddr, TcpStream};
use std::time::Duration;

const MAX_TOKEN_LENGTH: usize = 4096;
const MAX_MESSAGE_LENGTH: usize = 16 * 1024 * 1024;
const UNSUPPORTED: &str = "exact local browser profile/session/tab identity provider unavailable";

/// The complete identity that may be witnessed and later restored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserIdentity {
    pub process_id: i32,
    pub process_started_at: String,
    /// The browser's native window handle from the local DevTools protocol.
    /// This is not a title, array index, URL, or Evo registry id.
    pub native_window_identifier: String,
    /// Exact persistent profile argument pair witnessed on the browser
    /// process (`user-data-dir::profile-directory`).
    pub profile_token: String,
    /// Exact DevTools `browserContextId` reported for this target.
    pub session_token: String,
    /// Exact DevTools target id; this is the retained tab handle token.
    pub tab_token: String,
    pub url: String,
}

impl BrowserIdentity {
    /// Reads the exact browser constraints of a persisted capture surface.
    ///
    /// The browser-native window token stays in its own namespace: an AX
    /// identifier, registry id, title, or URL cannot replace a missing token.
    /// This is the same conversion used by the daemon-facing restore path.
    pub fn from_surface(
        surface: &evo_observation::surface_identity::SurfaceIdentity,
        url: &str,
    ) -> Result<Self, String> {
        if !surface.is_valid() {
            return Err("invalid witnessed browser surface identity".into());
        }
        if !matches!(
            crate::locator::classify_subject(url),
            crate::locator::LocatorKind::Url(_)
        ) {
            return Err("browser identity restoration requires the witnessed exact URL".into());
        }
        let (Some(window), Some(profile), Some(session), Some(tab)) = (
            &surface.browser_window,
            &surface.browser_profile,
            &surface.browser_session,
            &surface.browser_tab,
        ) else {
            return Err(
                "browser identity is incomplete; profile/session/window/tab constraints cannot be honored"
                    .into(),
            );
        };
        let identity = Self {
            process_id: surface.process_id,
            process_started_at: surface.process_started_at.clone(),
            native_window_identifier: window.clone(),
            profile_token: profile.clone(),
            session_token: session.clone(),
            tab_token: tab.clone(),
            url: url.to_string(),
        };
        validate_identity(&identity)?;
        Ok(identity)
    }

    pub fn is_valid(&self) -> bool {
        self.process_id > 0
            && valid_process_start(&self.process_started_at)
            && valid_token(&self.native_window_identifier)
            && valid_token(&self.profile_token)
            && valid_token(&self.session_token)
            && valid_token(&self.tab_token)
            && valid_token(&self.url)
    }
}

/// Whether a provider can honor the full identity contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserMacOSCapability {
    Unsupported,
    ExactIdentity,
}

/// A restoration receipt retains the observed postcondition separately from
/// an accepted native dispatch, just as the rest of the execution layer does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserRestoreReceipt {
    pub resource: String,
    pub dispatch: DispatchResult,
    pub verification: VerificationResult,
    pub postcondition: Option<BrowserIdentity>,
}

/// Provider-neutral exact browser identity contract.
///
/// `Tab` is a retained provider handle. Implementations must keep it tied to
/// one target through inspection and dispatch. They must not reconstruct it
/// from any field in [`BrowserIdentity`], especially not from its URL.
pub trait BrowserIdentityProvider {
    type Tab: Clone;

    fn capability(&self) -> BrowserMacOSCapability {
        BrowserMacOSCapability::Unsupported
    }

    fn process_started_at(&self, _pid: i32) -> Result<String, String> {
        Err(UNSUPPORTED.into())
    }

    /// Complete enumeration of inspectable browser tabs owned by `pid`.
    /// An incomplete enumeration is an error, never an empty list.
    fn tabs(&self, _pid: i32) -> Result<Vec<Self::Tab>, String> {
        Err(UNSUPPORTED.into())
    }

    fn evidence(&self, _tab: &Self::Tab) -> Result<BrowserIdentity, String> {
        Err(UNSUPPORTED.into())
    }

    /// Focus this exact retained tab; this must not navigate or open a URL.
    fn focus_tab(&self, _tab: &Self::Tab) -> Result<(), String> {
        Err(UNSUPPORTED.into())
    }

    /// Return the exact tab that is globally focused, not merely browser-local
    /// active state. A provider must refuse if it cannot prove this.
    fn globally_focused_tab(&self, _pid: i32) -> Result<Self::Tab, String> {
        Err(UNSUPPORTED.into())
    }
}

/// Witness the exact globally focused tab without deriving any identity from
/// a URL or title.
pub fn witness_focused_browser<P: BrowserIdentityProvider + ?Sized>(
    pid: i32,
    provider: &P,
) -> Result<BrowserIdentity, String> {
    require_exact_provider(provider)?;
    let tab = provider.globally_focused_tab(pid)?;
    let identity = provider.evidence(&tab)?;
    if identity.process_id != pid {
        return Err("focused browser tab belongs to an unexpected process".into());
    }
    validate_identity(&identity)?;
    if provider.process_started_at(pid)? != identity.process_started_at {
        return Err("browser process lifetime changed while witnessing identity".into());
    }
    Ok(identity)
}

/// Restore one exact witnessed browser identity.
///
/// The selected handle is retained from complete enumeration through focus.
/// A dispatch can be accepted while its postcondition fails; no retry or URL
/// fallback is attempted in either case.
pub fn restore_browser_identity<P: BrowserIdentityProvider + ?Sized>(
    identity: &BrowserIdentity,
    provider: &P,
) -> BrowserRestoreReceipt {
    let refused = |reason: String| BrowserRestoreReceipt {
        resource: identity.url.clone(),
        dispatch: DispatchResult::NotDispatched { reason },
        verification: VerificationResult::Unverified {
            reason: "no exact browser identity action was dispatched".into(),
        },
        postcondition: None,
    };

    if let Err(reason) = require_exact_provider(provider).and_then(|_| validate_identity(identity))
    {
        return refused(reason);
    }

    let check_process = || -> Result<(), String> {
        if provider.process_started_at(identity.process_id)? != identity.process_started_at {
            return Err(
                "browser process lifetime changed; witnessed PID was reused or restarted".into(),
            );
        }
        Ok(())
    };

    if let Err(reason) = check_process() {
        return refused(reason);
    }
    let tabs = match provider.tabs(identity.process_id) {
        Ok(tabs) => tabs,
        Err(reason) => return refused(reason),
    };

    let mut selected = None;
    let mut matches = 0usize;
    for (index, tab) in tabs.iter().enumerate() {
        let live = match provider.evidence(tab) {
            Ok(live) => live,
            Err(reason) => return refused(reason),
        };
        if let Err(reason) = validate_identity(&live) {
            return refused(format!("live tab identity is incomplete: {reason}"));
        }
        if live.process_id != identity.process_id {
            return refused("browser tab enumeration has an unexpected process owner".into());
        }
        // Identity tokens select; window/URL are constraints, not tie breakers.
        if live.profile_token == identity.profile_token
            && live.session_token == identity.session_token
            && live.tab_token == identity.tab_token
        {
            matches += 1;
            selected = Some(index);
        }
    }
    let index = match (matches, selected) {
        (1, Some(index)) => index,
        (0, _) => return refused("no live tab has the witnessed exact identity".into()),
        (count, _) => {
            return refused(format!(
                "{count} live tabs have the witnessed exact identity; exact identity is ambiguous"
            ));
        }
    };

    // Re-read this same handle immediately before dispatch. A stale or
    // replaced target must refuse rather than bind a replacement target.
    if let Err(reason) = check_process().and_then(|_| {
        let live = provider.evidence(&tabs[index])?;
        if live == *identity {
            Ok(())
        } else {
            Err("retained browser tab identity changed before dispatch".into())
        }
    }) {
        return refused(reason);
    }

    if let Err(reason) = provider.focus_tab(&tabs[index]) {
        return BrowserRestoreReceipt {
            resource: identity.url.clone(),
            dispatch: DispatchResult::Failed { reason },
            verification: VerificationResult::Unverified {
                reason: "exact browser focus dispatch was refused; no fallback was attempted"
                    .into(),
            },
            postcondition: None,
        };
    }

    let postcondition = (|| -> Result<BrowserIdentity, String> {
        check_process()?;
        let focused = provider.globally_focused_tab(identity.process_id)?;
        let live = provider.evidence(&focused)?;
        if live != *identity {
            return Err(
                "globally focused browser tab does not match the witnessed identity".into(),
            );
        }
        check_process()?;
        Ok(live)
    })();

    match postcondition {
        Ok(postcondition) => BrowserRestoreReceipt {
            resource: identity.url.clone(),
            dispatch: DispatchResult::RaiseAccepted,
            verification: VerificationResult::Verified,
            postcondition: Some(postcondition),
        },
        Err(reason) => BrowserRestoreReceipt {
            resource: identity.url.clone(),
            dispatch: DispatchResult::RaiseAccepted,
            verification: VerificationResult::Failed { reason },
            postcondition: None,
        },
    }
}

fn require_exact_provider<P: BrowserIdentityProvider + ?Sized>(provider: &P) -> Result<(), String> {
    if provider.capability() == BrowserMacOSCapability::ExactIdentity {
        Ok(())
    } else {
        Err(UNSUPPORTED.into())
    }
}

fn validate_identity(identity: &BrowserIdentity) -> Result<(), String> {
    if identity.is_valid() {
        Ok(())
    } else {
        Err("invalid or incomplete exact browser identity".into())
    }
}

fn valid_token(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= MAX_TOKEN_LENGTH && !value.contains('\0')
}

fn valid_process_start(value: &str) -> bool {
    if value.len() != 16
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return false;
    }
    let Ok(bits) = u64::from_str_radix(value, 16) else {
        return false;
    };
    let epoch = f64::from_bits(bits);
    epoch.is_finite() && epoch > 0.0
}

/// Explicitly configured loopback DevTools endpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalDevToolsConfig {
    pub address: IpAddr,
    pub port: u16,
    pub process_id: i32,
    pub timeout: Duration,
}

impl LocalDevToolsConfig {
    pub fn new(address: IpAddr, port: u16, process_id: i32) -> Result<Self, String> {
        if !address.is_loopback() {
            return Err(
                "browser DevTools address must be an explicitly configured loopback address".into(),
            );
        }
        if port == 0 || process_id <= 0 {
            return Err("browser DevTools port and process id must be positive".into());
        }
        Ok(Self {
            address,
            port,
            process_id,
            timeout: Duration::from_secs(2),
        })
    }

    fn socket(&self) -> SocketAddr {
        SocketAddr::new(self.address, self.port)
    }

    fn validate(&self) -> Result<(), String> {
        if !self.address.is_loopback()
            || self.port == 0
            || self.process_id <= 0
            || self.timeout.is_zero()
            || self.timeout > Duration::from_secs(30)
        {
            return Err("invalid explicit local DevTools configuration".into());
        }
        Ok(())
    }
}

/// A real local macOS browser adapter. Construction does not connect or start
/// anything; all work occurs only when a provider method is called.
#[derive(Debug, Clone)]
pub struct MacOSBrowserAdapter {
    config: LocalDevToolsConfig,
}

impl MacOSBrowserAdapter {
    pub fn new(config: LocalDevToolsConfig) -> Self {
        Self { config }
    }

    /// Builds an adapter only from an explicit local configuration. Nothing is
    /// discovered, started, or contacted here. Missing variables mean that
    /// browser restoration remains honestly unsupported.
    pub fn from_environment(process_id: i32) -> Result<Self, String> {
        let address = std::env::var("EVO_BROWSER_DEVTOOLS_ADDRESS")
            .map_err(|_| "EVO browser DevTools address is not explicitly configured".to_string())?
            .parse()
            .map_err(|_| "EVO browser DevTools address must be a literal IP".to_string())?;
        let port = std::env::var("EVO_BROWSER_DEVTOOLS_PORT")
            .map_err(|_| "EVO browser DevTools port is not explicitly configured".to_string())?
            .parse()
            .map_err(|_| "EVO browser DevTools port is invalid".to_string())?;
        let configured_pid = std::env::var("EVO_BROWSER_PID")
            .map_err(|_| "EVO browser PID is not explicitly configured".to_string())?
            .parse::<i32>()
            .map_err(|_| "EVO browser PID is invalid".to_string())?;
        if configured_pid != process_id {
            return Err("EVO browser PID differs from the witnessed browser process".into());
        }
        Ok(Self::new(LocalDevToolsConfig::new(
            address,
            port,
            configured_pid,
        )?))
    }

    pub fn config(&self) -> &LocalDevToolsConfig {
        &self.config
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalDevToolsTab {
    endpoint: SocketAddr,
    process_id: i32,
    process_started_at: String,
    profile_token: String,
    target_id: String,
    session_token: String,
    native_window_identifier: String,
    url: String,
    web_socket_url: String,
    browser_web_socket_url: String,
}

impl BrowserIdentityProvider for MacOSBrowserAdapter {
    type Tab = LocalDevToolsTab;

    fn capability(&self) -> BrowserMacOSCapability {
        if cfg!(target_os = "macos") && self.config.validate().is_ok() {
            BrowserMacOSCapability::ExactIdentity
        } else {
            BrowserMacOSCapability::Unsupported
        }
    }

    fn process_started_at(&self, pid: i32) -> Result<String, String> {
        self.config.validate()?;
        if pid != self.config.process_id {
            return Err("requested PID differs from the explicitly configured browser PID".into());
        }
        platform::process_started_at(pid)
    }

    fn tabs(&self, pid: i32) -> Result<Vec<Self::Tab>, String> {
        self.ensure_pid(pid)?;
        let process_started_at = self.process_started_at(pid)?;
        let browser_web_socket_url = self.browser_websocket_url()?;
        self.verify_endpoint_process(&browser_web_socket_url)?;
        let profile_token = platform::profile_token(pid)?;
        let targets = self.discover_targets(&profile_token)?;
        if self.browser_websocket_url()? != browser_web_socket_url
            || self.process_started_at(pid)? != process_started_at
        {
            return Err("browser process or DevTools session changed during enumeration".into());
        }
        targets
            .into_iter()
            .map(|target| {
                Ok(LocalDevToolsTab {
                    endpoint: self.config.socket(),
                    process_id: pid,
                    process_started_at: process_started_at.clone(),
                    profile_token: target.profile_token,
                    target_id: target.id,
                    session_token: target.session_token,
                    native_window_identifier: target.native_window_identifier,
                    url: target.url,
                    web_socket_url: target.web_socket_url,
                    browser_web_socket_url: browser_web_socket_url.clone(),
                })
            })
            .collect()
    }

    fn evidence(&self, tab: &Self::Tab) -> Result<BrowserIdentity, String> {
        self.validate_handle(tab)?;
        let process_started_at = self.process_started_at(tab.process_id)?;
        let browser_ws = self.browser_websocket_url()?;
        if browser_ws != tab.browser_web_socket_url {
            return Err("retained browser DevTools session changed or restarted".into());
        }
        self.verify_endpoint_process(&browser_ws)?;
        let live = self.target_by_id(&tab.target_id)?;
        if live.web_socket_url != tab.web_socket_url {
            return Err("retained browser tab WebSocket handle changed".into());
        }
        let identity = BrowserIdentity {
            process_id: tab.process_id,
            process_started_at,
            native_window_identifier: live.native_window_identifier,
            profile_token: live.profile_token,
            session_token: live.session_token,
            tab_token: tab.target_id.clone(),
            url: live.url,
        };
        if identity.process_started_at != tab.process_started_at
            || identity.profile_token != tab.profile_token
            || identity.native_window_identifier != tab.native_window_identifier
            || identity.session_token != tab.session_token
            || identity.url != tab.url
        {
            return Err("retained browser tab identity changed or was replaced".into());
        }
        validate_identity(&identity)?;
        Ok(identity)
    }

    fn focus_tab(&self, tab: &Self::Tab) -> Result<(), String> {
        self.validate_handle(tab)?;
        let before = self.evidence(tab)?;
        self.cdp_browser_command(
            "Target.activateTarget",
            object(&[("targetId", string(&before.tab_token))]),
        )?;
        // The target id is used directly. This call is not a URL navigation and
        // does not search for a replacement target.
        cdp_command(
            &tab.web_socket_url,
            "Page.bringToFront",
            object(&[]),
            &self.config,
        )?;
        Ok(())
    }

    fn globally_focused_tab(&self, pid: i32) -> Result<Self::Tab, String> {
        self.ensure_pid(pid)?;
        if platform::frontmost_pid()? != pid {
            return Err("the browser process is not the globally focused application".into());
        }
        let tabs = self.tabs(pid)?;
        let mut focused = None;
        for tab in tabs {
            // A separate isolated world prevents website JavaScript from
            // overriding document.hasFocus() and forging focus evidence.
            let value = self.isolated_focus(&tab)?;
            let has_focus = value
                .object_field("result")
                .and_then(|result| result.object_field("value"))
                .and_then(JsonValue::as_bool)
                .ok_or_else(|| "browser did not expose an exact tab focus witness".to_string())?;
            if has_focus {
                if focused.is_some() {
                    return Err(
                        "multiple browser tabs report focus; exact global tab is ambiguous".into(),
                    );
                }
                focused = Some(tab);
            }
        }
        if platform::frontmost_pid()? != pid {
            return Err("globally focused application changed during tab inspection".into());
        }
        focused.ok_or_else(|| "no browser tab exposes the globally focused document".into())
    }
}

impl MacOSBrowserAdapter {
    fn ensure_pid(&self, pid: i32) -> Result<(), String> {
        self.config.validate()?;
        if pid == self.config.process_id {
            Ok(())
        } else {
            Err("requested PID differs from the explicitly configured browser PID".into())
        }
    }

    fn validate_handle(&self, tab: &LocalDevToolsTab) -> Result<(), String> {
        self.config.validate()?;
        if tab.endpoint != self.config.socket() || tab.process_id != self.config.process_id {
            return Err(
                "retained browser handle belongs to a different configured endpoint".into(),
            );
        }
        Ok(())
    }

    fn discover_targets(&self, profile_token: &str) -> Result<Vec<DiscoveredTarget>, String> {
        let listed = self.list_targets()?;
        let browser_ws = self.browser_websocket_url()?;
        let response = cdp_command(&browser_ws, "Target.getTargets", object(&[]), &self.config)?;
        let infos = response
            .object_field("targetInfos")
            .and_then(JsonValue::as_array)
            .ok_or_else(|| "DevTools omitted complete target information".to_string())?;
        let mut discovered = Vec::new();
        for info in infos {
            if info.object_field("type").and_then(JsonValue::as_str) != Some("page") {
                continue;
            }
            let id = required_json_string(info, "targetId")?;
            let listed_target = listed
                .iter()
                .find(|target| target.id == id)
                .ok_or_else(|| {
                    "DevTools target list changed during exact enumeration".to_string()
                })?;
            let session_token = required_json_string(info, "browserContextId")?;
            let url = required_json_string(info, "url")?;
            if url != listed_target.url {
                return Err("DevTools reported inconsistent URL evidence for one target".into());
            }
            let window_id = self.window_id(&id)?;
            discovered.push(DiscoveredTarget {
                id,
                session_token,
                profile_token: profile_token.into(),
                native_window_identifier: window_id,
                url,
                web_socket_url: listed_target.web_socket_url.clone(),
            });
        }
        Ok(discovered)
    }

    fn target_by_id(&self, target_id: &str) -> Result<DiscoveredTarget, String> {
        let listed = self.list_targets()?;
        let listed_target = listed
            .iter()
            .find(|target| target.id == target_id)
            .ok_or_else(|| "retained browser target is stale or closed".to_string())?;
        let browser_ws = self.browser_websocket_url()?;
        let response = cdp_command(
            &browser_ws,
            "Target.getTargetInfo",
            object(&[("targetId", string(target_id))]),
            &self.config,
        )?;
        let info = response
            .object_field("targetInfo")
            .ok_or_else(|| "DevTools omitted exact target information".to_string())?;
        if info.object_field("type").and_then(JsonValue::as_str) != Some("page")
            || required_json_string(info, "targetId")? != target_id
        {
            return Err("retained browser target changed type or identity".into());
        }
        let url = required_json_string(info, "url")?;
        let profile_token = platform::profile_token(self.config.process_id)?;
        let session_token = required_json_string(info, "browserContextId")?;
        if url != listed_target.url {
            return Err("retained browser target URL changed".into());
        }
        Ok(DiscoveredTarget {
            id: target_id.into(),
            session_token,
            profile_token,
            native_window_identifier: self.window_id(target_id)?,
            url,
            web_socket_url: listed_target.web_socket_url.clone(),
        })
    }

    fn list_targets(&self) -> Result<Vec<ListedTarget>, String> {
        let body = http_get(&self.config, "/json/list")?;
        let value = parse_json(&body)?;
        let values = value
            .as_array()
            .ok_or_else(|| "DevTools /json/list response is not an array".to_string())?;
        values
            .iter()
            .filter(|value| value.object_field("type").and_then(JsonValue::as_str) == Some("page"))
            .map(|value| {
                Ok(ListedTarget {
                    id: required_json_string(value, "id")?,
                    url: required_json_string(value, "url")?,
                    web_socket_url: {
                        let url = required_json_string(value, "webSocketDebuggerUrl")?;
                        validate_websocket_url(&url, &self.config)?;
                        url
                    },
                })
            })
            .collect()
    }

    fn browser_websocket_url(&self) -> Result<String, String> {
        let body = http_get(&self.config, "/json/version")?;
        let value = parse_json(&body)?;
        let url = required_json_string(&value, "webSocketDebuggerUrl")?;
        validate_websocket_url(&url, &self.config)?;
        Ok(url)
    }

    fn window_id(&self, target_id: &str) -> Result<String, String> {
        let browser_ws = self.browser_websocket_url()?;
        let response = cdp_command(
            &browser_ws,
            "Browser.getWindowForTarget",
            object(&[("targetId", string(target_id))]),
            &self.config,
        )?;
        let window_id = response
            .object_field("windowId")
            .and_then(JsonValue::as_number)
            .ok_or_else(|| "DevTools omitted the native browser window identifier".to_string())?;
        if window_id.parse::<u32>().is_err() {
            return Err("browser native window id is not an exact unsigned integer".into());
        }
        Ok(window_id.into())
    }

    fn cdp_browser_command(&self, method: &str, params: String) -> Result<JsonValue, String> {
        let browser_ws = self.browser_websocket_url()?;
        cdp_command(&browser_ws, method, params, &self.config)
    }

    fn verify_endpoint_process(&self, browser_ws: &str) -> Result<(), String> {
        let response = cdp_command(
            browser_ws,
            "SystemInfo.getProcessInfo",
            object(&[]),
            &self.config,
        )?;
        let processes = response
            .object_field("processInfo")
            .and_then(JsonValue::as_array)
            .ok_or_else(|| "DevTools cannot witness its browser process owner".to_string())?;
        let browser_processes = processes
            .iter()
            .filter(|process| {
                process.object_field("type").and_then(JsonValue::as_str) == Some("browser")
            })
            .collect::<Vec<_>>();
        let configured_pid = self.config.process_id.to_string();
        if browser_processes.len() != 1
            || browser_processes[0]
                .object_field("id")
                .and_then(JsonValue::as_number)
                != Some(configured_pid.as_str())
        {
            return Err(
                "DevTools endpoint is not owned by the configured exact browser PID".into(),
            );
        }
        Ok(())
    }

    fn isolated_focus(&self, tab: &LocalDevToolsTab) -> Result<JsonValue, String> {
        let mut socket = cdp_socket(&tab.web_socket_url, &self.config)?;
        let tree = socket.command(1, "Page.getFrameTree", object(&[]))?;
        let frame_id = tree
            .object_field("frameTree")
            .and_then(|value| value.object_field("frame"))
            .and_then(|value| value.object_field("id"))
            .and_then(JsonValue::as_str)
            .filter(|value| valid_token(value))
            .ok_or_else(|| "exact top-level browser frame is unavailable".to_string())?;
        let world = socket.command(
            2,
            "Page.createIsolatedWorld",
            object(&[("frameId", string(frame_id))]),
        )?;
        let context_id = world
            .object_field("executionContextId")
            .and_then(JsonValue::as_number)
            .filter(|value| value.parse::<u32>().is_ok())
            .ok_or_else(|| "browser cannot provide an isolated focus witness".to_string())?;
        let result = socket.command(
            3,
            "Runtime.evaluate",
            object(&[
                ("expression", string("document.hasFocus()")),
                ("contextId", context_id.into()),
                ("returnByValue", "true".into()),
            ]),
        )?;
        if result.object_field("exceptionDetails").is_some() {
            return Err("isolated browser focus probe failed".into());
        }
        Ok(result)
    }
}

#[derive(Debug, Clone)]
struct ListedTarget {
    id: String,
    url: String,
    web_socket_url: String,
}

#[derive(Debug, Clone)]
struct DiscoveredTarget {
    id: String,
    session_token: String,
    profile_token: String,
    native_window_identifier: String,
    url: String,
    web_socket_url: String,
}

fn http_get(config: &LocalDevToolsConfig, path: &str) -> Result<String, String> {
    config.validate()?;
    if !path.starts_with('/') || path.contains('\r') || path.contains('\n') {
        return Err("invalid local DevTools path".into());
    }
    let mut stream = TcpStream::connect_timeout(&config.socket(), config.timeout)
        .map_err(|error| format!("local DevTools endpoint unavailable: {error}"))?;
    stream
        .set_read_timeout(Some(config.timeout))
        .map_err(|error| error.to_string())?;
    stream
        .set_write_timeout(Some(config.timeout))
        .map_err(|error| error.to_string())?;
    let host = match config.address {
        IpAddr::V4(address) => address.to_string(),
        IpAddr::V6(address) => format!("[{address}]"),
    };
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: {host}:{}\r\nConnection: close\r\n\r\n",
        config.port
    )
    .map_err(|error| error.to_string())?;
    let mut bytes = Vec::new();
    Read::by_ref(&mut stream)
        .take((MAX_MESSAGE_LENGTH + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("local DevTools response unreadable: {error}"))?;
    if bytes.len() > MAX_MESSAGE_LENGTH {
        return Err("DevTools response exceeds the local adapter limit".into());
    }
    let text =
        String::from_utf8(bytes).map_err(|_| "local DevTools response is not UTF-8".to_string())?;
    let (headers, body) = text
        .split_once("\r\n\r\n")
        .ok_or_else(|| "local DevTools response has no HTTP header boundary".to_string())?;
    if !headers
        .lines()
        .next()
        .is_some_and(|line| line.split_whitespace().nth(1) == Some("200"))
    {
        return Err("local DevTools endpoint refused the exact request".into());
    }
    Ok(body.into())
}

fn validate_websocket_url(url: &str, config: &LocalDevToolsConfig) -> Result<(), String> {
    config.validate()?;
    let (host, port, _) = parse_ws_url(url)?;
    if host != config.address || port != config.port {
        return Err(
            "DevTools WebSocket escaped the explicitly configured loopback endpoint".into(),
        );
    }
    Ok(())
}

fn parse_ws_url(url: &str) -> Result<(IpAddr, u16, &str), String> {
    let rest = url
        .strip_prefix("ws://")
        .ok_or_else(|| "only unencrypted loopback DevTools WebSockets are supported".to_string())?;
    let (authority, path) = rest
        .split_once('/')
        .ok_or_else(|| "DevTools WebSocket URL has no path".to_string())?;
    let (host_text, port) = if let Some(host) = authority.strip_prefix('[') {
        let (host, port) = host
            .split_once("]:")
            .ok_or_else(|| "invalid IPv6 DevTools WebSocket authority".to_string())?;
        (host, port)
    } else {
        authority
            .rsplit_once(':')
            .ok_or_else(|| "DevTools WebSocket authority has no port".to_string())?
    };
    let port = port
        .parse::<u16>()
        .map_err(|_| "invalid DevTools WebSocket port".to_string())?;
    let host = host_text
        .parse::<IpAddr>()
        .map_err(|_| "DevTools WebSocket host must be a literal loopback IP".to_string())?;
    if !host.is_loopback()
        || path.is_empty()
        || path
            .bytes()
            .any(|byte| byte.is_ascii_control() || byte.is_ascii_whitespace())
    {
        return Err("DevTools WebSocket must use a literal loopback IP and path".into());
    }
    Ok((host, port, path))
}

fn cdp_command(
    websocket_url: &str,
    method: &str,
    params: String,
    config: &LocalDevToolsConfig,
) -> Result<JsonValue, String> {
    let mut socket = cdp_socket(websocket_url, config)?;
    socket.command(1, method, params)
}

fn cdp_socket(websocket_url: &str, config: &LocalDevToolsConfig) -> Result<WebSocket, String> {
    validate_websocket_url(websocket_url, config)?;
    let (_, _, path) = parse_ws_url(websocket_url)?;
    let stream = TcpStream::connect_timeout(
        &SocketAddr::new(config.address, config.port),
        config.timeout,
    )
    .map_err(|error| format!("local DevTools endpoint unavailable: {error}"))?;
    WebSocket::connect(stream, path, config)
}

struct WebSocket {
    stream: TcpStream,
    fragment: Vec<u8>,
}

impl WebSocket {
    fn command(&mut self, id: u32, method: &str, params: String) -> Result<JsonValue, String> {
        self.send_text(&format!(
            "{{\"id\":{id},\"method\":{},\"params\":{params}}}",
            string(method)
        ))?;
        for _ in 0..1024 {
            let value = parse_json(&self.receive_text()?)?;
            if value.object_field("id").and_then(JsonValue::as_number)
                != Some(id.to_string().as_str())
            {
                continue;
            }
            if value.object_field("error").is_some() {
                return Err(format!("DevTools refused the exact {method} command"));
            }
            return value
                .object_field("result")
                .cloned()
                .ok_or_else(|| format!("DevTools omitted the result for {method}"));
        }
        Err("DevTools exact command response was not received".into())
    }
    fn connect(
        mut stream: TcpStream,
        path: &str,
        config: &LocalDevToolsConfig,
    ) -> Result<Self, String> {
        stream
            .set_read_timeout(Some(config.timeout))
            .and_then(|_| stream.set_write_timeout(Some(config.timeout)))
            .map_err(|error| error.to_string())?;
        let key = base64_encode(&random_bytes::<16>()?);
        let host = match config.address {
            IpAddr::V4(address) => address.to_string(),
            IpAddr::V6(address) => format!("[{address}]"),
        };
        write!(
            stream,
            "GET /{path} HTTP/1.1\r\nHost: {host}:{}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: {key}\r\nSec-WebSocket-Version: 13\r\n\r\n",
            config.port
        )
        .map_err(|error| error.to_string())?;
        let mut response = Vec::new();
        let mut byte = [0u8; 1];
        while response.len() < 64 * 1024 {
            stream
                .read_exact(&mut byte)
                .map_err(|error| error.to_string())?;
            response.push(byte[0]);
            if response.ends_with(b"\r\n\r\n") {
                break;
            }
        }
        let response = String::from_utf8(response)
            .map_err(|_| "WebSocket handshake is not UTF-8".to_string())?;
        if !response
            .lines()
            .next()
            .is_some_and(|line| line.contains(" 101 "))
        {
            return Err("local DevTools WebSocket handshake was refused".into());
        }
        let accept = response
            .lines()
            .find_map(|line| {
                line.split_once(':')
                    .filter(|(name, _)| name.eq_ignore_ascii_case("Sec-WebSocket-Accept"))
                    .map(|(_, value)| value.trim())
            })
            .ok_or_else(|| "DevTools omitted the WebSocket accept header".to_string())?;
        let expected = base64_encode(&sha1(
            format!("{key}258EAFA5-E914-47DA-95CA-C5AB0DC85B11").as_bytes(),
        ));
        if accept != expected {
            return Err(
                "DevTools WebSocket accept header did not match the configured endpoint".into(),
            );
        }
        Ok(Self {
            stream,
            fragment: Vec::new(),
        })
    }

    fn send_text(&mut self, text: &str) -> Result<(), String> {
        let payload = text.as_bytes();
        if payload.len() > MAX_MESSAGE_LENGTH {
            return Err("DevTools command exceeds the local adapter frame limit".into());
        }
        let mask = random_bytes::<4>()?;
        let mut frame = vec![0x81];
        if payload.len() < 126 {
            frame.push(0x80 | payload.len() as u8);
        } else if payload.len() <= u16::MAX as usize {
            frame.push(0x80 | 126);
            frame.extend_from_slice(&(payload.len() as u16).to_be_bytes());
        } else {
            frame.push(0x80 | 127);
            frame.extend_from_slice(&(payload.len() as u64).to_be_bytes());
        }
        frame.extend_from_slice(&mask);
        frame.extend(
            payload
                .iter()
                .enumerate()
                .map(|(index, byte)| byte ^ mask[index % 4]),
        );
        self.stream
            .write_all(&frame)
            .map_err(|error| error.to_string())
    }

    fn receive_text(&mut self) -> Result<String, String> {
        loop {
            let (opcode, fin, payload) = self.receive_frame()?;
            match opcode {
                0x1 => {
                    if !self.fragment.is_empty() {
                        return Err("unexpected DevTools fragmented message".into());
                    }
                    self.fragment.extend_from_slice(&payload);
                    if fin {
                        let value = std::mem::take(&mut self.fragment);
                        return String::from_utf8(value)
                            .map_err(|_| "DevTools message is not UTF-8".into());
                    }
                }
                0x0 => {
                    self.fragment.extend_from_slice(&payload);
                    if fin {
                        let value = std::mem::take(&mut self.fragment);
                        return String::from_utf8(value)
                            .map_err(|_| "DevTools message is not UTF-8".into());
                    }
                }
                0x9 => self.send_control(0xA, &payload)?,
                0x8 => return Err("DevTools WebSocket closed before the exact response".into()),
                0xA => {}
                _ => return Err("unsupported DevTools WebSocket frame".into()),
            }
            if self.fragment.len() > MAX_MESSAGE_LENGTH {
                return Err("DevTools message exceeds the local adapter limit".into());
            }
        }
    }

    fn receive_frame(&mut self) -> Result<(u8, bool, Vec<u8>), String> {
        let mut header = [0u8; 2];
        self.stream
            .read_exact(&mut header)
            .map_err(|error| error.to_string())?;
        let fin = header[0] & 0x80 != 0;
        let opcode = header[0] & 0x0f;
        let masked = header[1] & 0x80 != 0;
        if masked || header[0] & 0x70 != 0 {
            return Err("invalid server WebSocket frame".into());
        }
        let mut length = (header[1] & 0x7f) as usize;
        if length == 126 {
            let mut bytes = [0u8; 2];
            self.stream
                .read_exact(&mut bytes)
                .map_err(|error| error.to_string())?;
            length = u16::from_be_bytes(bytes) as usize;
        } else if length == 127 {
            let mut bytes = [0u8; 8];
            self.stream
                .read_exact(&mut bytes)
                .map_err(|error| error.to_string())?;
            let value = u64::from_be_bytes(bytes);
            length =
                usize::try_from(value).map_err(|_| "DevTools frame is too large".to_string())?;
        }
        if length > MAX_MESSAGE_LENGTH {
            return Err("DevTools frame exceeds the local adapter limit".into());
        }
        let mut mask = [0u8; 4];
        if masked {
            self.stream
                .read_exact(&mut mask)
                .map_err(|error| error.to_string())?;
        }
        let mut payload = vec![0u8; length];
        self.stream
            .read_exact(&mut payload)
            .map_err(|error| error.to_string())?;
        if masked {
            for (index, byte) in payload.iter_mut().enumerate() {
                *byte ^= mask[index % 4];
            }
        }
        Ok((opcode, fin, payload))
    }

    fn send_control(&mut self, opcode: u8, payload: &[u8]) -> Result<(), String> {
        if payload.len() > 125 {
            return Err("DevTools control frame is too large".into());
        }
        let mask = random_bytes::<4>()?;
        let mut frame = vec![0x80 | opcode, 0x80 | payload.len() as u8];
        frame.extend_from_slice(&mask);
        frame.extend(
            payload
                .iter()
                .enumerate()
                .map(|(index, byte)| byte ^ mask[index % 4]),
        );
        self.stream
            .write_all(&frame)
            .map_err(|error| error.to_string())
    }
}

fn random_bytes<const N: usize>() -> Result<[u8; N], String> {
    let mut bytes = [0u8; N];
    File::open("/dev/urandom")
        .and_then(|mut file| file.read_exact(&mut bytes))
        .map_err(|_| "local OS randomness is unavailable".to_string())?;
    Ok(bytes)
}

impl Drop for WebSocket {
    fn drop(&mut self) {
        let _ = self.stream.shutdown(Shutdown::Both);
    }
}

#[derive(Debug, Clone, PartialEq)]
enum JsonValue {
    Null,
    Bool(bool),
    Number(String),
    String(String),
    Array(Vec<JsonValue>),
    Object(BTreeMap<String, JsonValue>),
}

impl JsonValue {
    fn as_array(&self) -> Option<&[JsonValue]> {
        match self {
            Self::Array(values) => Some(values),
            _ => None,
        }
    }

    fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(*value),
            _ => None,
        }
    }

    fn as_number(&self) -> Option<&str> {
        match self {
            Self::Number(value) => Some(value),
            _ => None,
        }
    }

    fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            _ => None,
        }
    }

    fn object_field(&self, key: &str) -> Option<&JsonValue> {
        match self {
            Self::Object(values) => values.get(key),
            _ => None,
        }
    }
}

fn required_json_string(value: &JsonValue, key: &str) -> Result<String, String> {
    value
        .object_field(key)
        .and_then(JsonValue::as_str)
        .filter(|value| valid_token(value))
        .map(str::to_owned)
        .ok_or_else(|| format!("DevTools omitted exact non-empty {key} evidence"))
}

fn string(value: &str) -> String {
    format!("\"{}\"", json_escape(value))
}

fn object(fields: &[(&str, String)]) -> String {
    let fields = fields
        .iter()
        .map(|(key, value)| format!("{}:{value}", string(key)))
        .collect::<Vec<_>>();
    format!("{{{}}}", fields.join(","))
}

fn json_escape(value: &str) -> String {
    value
        .chars()
        .flat_map(|character| match character {
            '"' => "\\\"".chars().collect::<Vec<_>>(),
            '\\' => "\\\\".chars().collect(),
            '\n' => "\\n".chars().collect(),
            '\r' => "\\r".chars().collect(),
            '\t' => "\\t".chars().collect(),
            character if character.is_control() => {
                format!("\\u{:04x}", character as u32).chars().collect()
            }
            character => vec![character],
        })
        .collect()
}

struct JsonParser<'a> {
    input: &'a str,
    index: usize,
}

fn parse_json(input: &str) -> Result<JsonValue, String> {
    let mut parser = JsonParser { input, index: 0 };
    let value = parser.value()?;
    parser.ws();
    if parser.index != input.len() {
        return Err("trailing bytes after JSON value".into());
    }
    Ok(value)
}

impl<'a> JsonParser<'a> {
    fn value(&mut self) -> Result<JsonValue, String> {
        self.ws();
        match self.peek() {
            Some('n') => {
                self.expect("null")?;
                Ok(JsonValue::Null)
            }
            Some('t') => {
                self.expect("true")?;
                Ok(JsonValue::Bool(true))
            }
            Some('f') => {
                self.expect("false")?;
                Ok(JsonValue::Bool(false))
            }
            Some('"') => Ok(JsonValue::String(self.string()?)),
            Some('[') => self.array(),
            Some('{') => self.object(),
            Some(character) if character == '-' || character.is_ascii_digit() => {
                Ok(JsonValue::Number(self.number()?))
            }
            _ => Err("invalid JSON value".into()),
        }
    }

    fn array(&mut self) -> Result<JsonValue, String> {
        self.byte('[')?;
        let mut values = Vec::new();
        loop {
            self.ws();
            if self.consume(']') {
                return Ok(JsonValue::Array(values));
            }
            values.push(self.value()?);
            self.ws();
            if self.consume(']') {
                return Ok(JsonValue::Array(values));
            }
            self.byte(',')?;
        }
    }

    fn object(&mut self) -> Result<JsonValue, String> {
        self.byte('{')?;
        let mut values = BTreeMap::new();
        loop {
            self.ws();
            if self.consume('}') {
                return Ok(JsonValue::Object(values));
            }
            let key = self.string()?;
            self.ws();
            self.byte(':')?;
            let value = self.value()?;
            values.insert(key, value);
            self.ws();
            if self.consume('}') {
                return Ok(JsonValue::Object(values));
            }
            self.byte(',')?;
        }
    }

    fn string(&mut self) -> Result<String, String> {
        self.byte('"')?;
        let mut output = String::new();
        while let Some(character) = self.next() {
            match character {
                '"' => return Ok(output),
                '\\' => match self
                    .next()
                    .ok_or_else(|| "unterminated JSON escape".to_string())?
                {
                    '"' => output.push('"'),
                    '\\' => output.push('\\'),
                    '/' => output.push('/'),
                    'b' => output.push('\u{0008}'),
                    'f' => output.push('\u{000c}'),
                    'n' => output.push('\n'),
                    'r' => output.push('\r'),
                    't' => output.push('\t'),
                    'u' => {
                        let mut value = 0u32;
                        for _ in 0..4 {
                            let byte = self
                                .next()
                                .ok_or_else(|| "short JSON unicode escape".to_string())?;
                            value = value * 16
                                + byte
                                    .to_digit(16)
                                    .ok_or_else(|| "invalid JSON unicode escape".to_string())?;
                        }
                        output.push(
                            char::from_u32(value)
                                .ok_or_else(|| "invalid JSON code point".to_string())?,
                        );
                    }
                    _ => return Err("invalid JSON escape".into()),
                },
                character if character.is_control() => {
                    return Err("control byte in JSON string".into());
                }
                character => output.push(character),
            }
        }
        Err("unterminated JSON string".into())
    }

    fn number(&mut self) -> Result<String, String> {
        let start = self.index;
        while let Some(character) = self.peek() {
            if character.is_ascii_digit() || matches!(character, '-' | '+' | '.' | 'e' | 'E') {
                self.next();
            } else {
                break;
            }
        }
        let value = &self.input[start..self.index];
        if value.is_empty() {
            Err("empty JSON number".into())
        } else {
            Ok(value.into())
        }
    }

    fn ws(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.next();
        }
    }

    fn expect(&mut self, value: &str) -> Result<(), String> {
        for expected in value.chars() {
            if self.next() != Some(expected) {
                return Err("invalid JSON literal".into());
            }
        }
        Ok(())
    }

    fn byte(&mut self, expected: char) -> Result<(), String> {
        if self.next() == Some(expected) {
            Ok(())
        } else {
            Err(format!("expected JSON {expected:?}"))
        }
    }

    fn consume(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.next();
            true
        } else {
            false
        }
    }

    fn peek(&self) -> Option<char> {
        self.input[self.index..].chars().next()
    }

    fn next(&mut self) -> Option<char> {
        let character = self.peek()?;
        self.index += character.len_utf8();
        Some(character)
    }
}

fn sha1(input: &[u8]) -> [u8; 20] {
    let mut message = input.to_vec();
    let bit_length = (message.len() as u64) * 8;
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_length.to_be_bytes());
    let mut h = [
        0x67452301u32,
        0xefcdab89,
        0x98badcfe,
        0x10325476,
        0xc3d2e1f0,
    ];
    for chunk in message.chunks_exact(64) {
        let mut words = [0u32; 80];
        for (index, word) in words[..16].iter_mut().enumerate() {
            *word = u32::from_be_bytes(chunk[index * 4..index * 4 + 4].try_into().unwrap());
        }
        for index in 16..80 {
            words[index] =
                (words[index - 3] ^ words[index - 8] ^ words[index - 14] ^ words[index - 16])
                    .rotate_left(1);
        }
        let (mut a, mut b, mut c, mut d, mut e) = (h[0], h[1], h[2], h[3], h[4]);
        for (index, word) in words.iter().enumerate() {
            let (f, k) = match index {
                0..=19 => ((b & c) | ((!b) & d), 0x5a827999),
                20..=39 => (b ^ c ^ d, 0x6ed9eba1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8f1bbcdc),
                _ => (b ^ c ^ d, 0xca62c1d6),
            };
            let next = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(*word);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = next;
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
    }
    let mut output = [0u8; 20];
    for (index, value) in h.iter().enumerate() {
        output[index * 4..index * 4 + 4].copy_from_slice(&value.to_be_bytes());
    }
    output
}

fn base64_encode(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::new();
    for chunk in input.chunks(3) {
        let first = chunk[0];
        output.push(TABLE[(first >> 2) as usize] as char);
        let second = chunk.get(1).copied().unwrap_or(0);
        output.push(TABLE[((first & 0x03) << 4 | second >> 4) as usize] as char);
        if chunk.len() > 1 {
            output.push(
                TABLE[((second & 0x0f) << 2 | chunk.get(2).copied().unwrap_or(0) >> 6) as usize]
                    as char,
            );
        } else {
            output.push('=');
        }
        if chunk.len() > 2 {
            output.push(TABLE[(chunk[2] & 0x3f) as usize] as char);
        } else {
            output.push('=');
        }
    }
    output
}

#[cfg(target_os = "macos")]
mod platform {
    use super::{valid_process_start, valid_token};
    use std::ffi::CStr;
    use std::os::raw::{c_char, c_void};
    use std::process::Command;

    type Id = *mut Object;
    type Class = *mut Object;
    type Sel = *mut c_void;

    #[repr(C)]
    struct Object {
        _private: [u8; 0],
    }

    #[link(name = "objc")]
    #[link(name = "Foundation", kind = "framework")]
    #[link(name = "AppKit", kind = "framework")]
    unsafe extern "C" {
        fn objc_getClass(name: *const c_char) -> Class;
        fn objc_msgSend();
        fn sel_registerName(name: *const c_char) -> Sel;
    }

    pub fn process_started_at(pid: i32) -> Result<String, String> {
        if pid <= 0 {
            return Err("browser PID is not positive".into());
        }
        unsafe {
            let app = msg_id_i32(
                class(c"NSRunningApplication"),
                selector(c"runningApplicationWithProcessIdentifier:"),
                pid,
            );
            if app.is_null() {
                return Err("witnessed browser process is not running".into());
            }
            let date = msg_id(app, selector(c"launchDate"));
            if date.is_null() {
                return Err("browser launch date is unavailable".into());
            }
            let seconds = msg_f64(date, selector(c"timeIntervalSince1970"));
            if !seconds.is_finite() || seconds <= 0.0 {
                return Err("browser launch date is unreadable".into());
            }
            let token = format!("{:016x}", seconds.to_bits());
            if valid_process_start(&token) {
                Ok(token)
            } else {
                Err("browser launch token is invalid".into())
            }
        }
    }

    pub fn frontmost_pid() -> Result<i32, String> {
        unsafe {
            let workspace = msg_id(class(c"NSWorkspace"), selector(c"sharedWorkspace"));
            if workspace.is_null() {
                return Err("NSWorkspace is unavailable".into());
            }
            let application = msg_id(workspace, selector(c"frontmostApplication"));
            if application.is_null() {
                return Err("globally focused application is unavailable".into());
            }
            let pid = msg_i32(application, selector(c"processIdentifier"));
            if pid > 0 {
                Ok(pid)
            } else {
                Err("globally focused application PID is unavailable".into())
            }
        }
    }

    pub fn profile_token(pid: i32) -> Result<String, String> {
        let output = Command::new("/bin/ps")
            .args(["-ww", "-p", &pid.to_string(), "-o", "command="])
            .output()
            .map_err(|error| {
                format!("cannot inspect the local browser process command line: {error}")
            })?;
        if !output.status.success() {
            return Err("browser process command line is unavailable".into());
        }
        let command = String::from_utf8(output.stdout)
            .map_err(|_| "browser process command line is not UTF-8".to_string())?;
        let args = shell_words(&command);
        let user_data_dir = option_arg(&args, "--user-data-dir").ok_or_else(|| {
            "browser profile path is not exposed by the local process".to_string()
        })?;
        let profile = option_arg(&args, "--profile-directory").ok_or_else(|| {
            "browser profile directory is not exposed by the local process".to_string()
        })?;
        if !std::path::Path::new(&user_data_dir).is_absolute()
            || !valid_token(&user_data_dir)
            || !valid_token(&profile)
        {
            return Err("browser profile evidence is missing or not absolute".into());
        }
        Ok(format!("{user_data_dir}::{profile}"))
    }

    fn option_arg(args: &[String], name: &str) -> Option<String> {
        args.iter()
            .find_map(|arg| arg.strip_prefix(&format!("{name}=")).map(str::to_owned))
            .or_else(|| {
                args.windows(2)
                    .find(|pair| pair[0] == name)
                    .map(|pair| pair[1].clone())
            })
    }

    fn shell_words(value: &str) -> Vec<String> {
        let mut words = Vec::new();
        let mut word = String::new();
        let mut quote = None;
        let mut escaped = false;
        for character in value.chars() {
            if escaped {
                word.push(character);
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if quote == Some(character) {
                quote = None;
            } else if quote.is_none() && (character == '\'' || character == '"') {
                quote = Some(character);
            } else if quote.is_none() && character.is_whitespace() {
                if !word.is_empty() {
                    words.push(std::mem::take(&mut word));
                }
            } else {
                word.push(character);
            }
        }
        if escaped {
            word.push('\\');
        }
        if !word.is_empty() {
            words.push(word);
        }
        words
    }

    unsafe fn class(name: &CStr) -> Class {
        unsafe { objc_getClass(name.as_ptr()) }
    }

    unsafe fn selector(name: &CStr) -> Sel {
        unsafe { sel_registerName(name.as_ptr()) }
    }

    unsafe fn msg_id(receiver: Id, selector: Sel) -> Id {
        let function: extern "C" fn(Id, Sel) -> Id =
            unsafe { std::mem::transmute(objc_msgSend as *const ()) };
        function(receiver, selector)
    }

    unsafe fn msg_id_i32(receiver: Id, selector: Sel, value: i32) -> Id {
        let function: extern "C" fn(Id, Sel, i32) -> Id =
            unsafe { std::mem::transmute(objc_msgSend as *const ()) };
        function(receiver, selector, value)
    }

    unsafe fn msg_i32(receiver: Id, selector: Sel) -> i32 {
        let function: extern "C" fn(Id, Sel) -> i32 =
            unsafe { std::mem::transmute(objc_msgSend as *const ()) };
        function(receiver, selector)
    }

    unsafe fn msg_f64(receiver: Id, selector: Sel) -> f64 {
        let function: extern "C" fn(Id, Sel) -> f64 =
            unsafe { std::mem::transmute(objc_msgSend as *const ()) };
        function(receiver, selector)
    }
}

#[cfg(not(target_os = "macos"))]
mod platform {

    pub fn process_started_at(_pid: i32) -> Result<String, String> {
        Err("the local browser adapter is only supported on macOS".into())
    }

    pub fn frontmost_pid() -> Result<i32, String> {
        Err("global browser focus is only supported on macOS".into())
    }

    pub fn profile_token(_pid: i32) -> Result<String, String> {
        Err("browser profile inspection is only supported on macOS".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};

    const URL: &str = "https://example.test/exact?tab=7#section";

    fn identity() -> BrowserIdentity {
        BrowserIdentity {
            process_id: 77,
            process_started_at: format!("{:016x}", 1_791_000_000.5f64.to_bits()),
            native_window_identifier: "cdp-window:4".into(),
            profile_token: "/Users/test/Chrome::Default".into(),
            session_token: "session-a".into(),
            tab_token: "tab-a".into(),
            url: URL.into(),
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct Handle(usize);

    struct FakeProvider {
        capability: BrowserMacOSCapability,
        live: Vec<BrowserIdentity>,
        focused: Result<usize, String>,
        focused_live: Option<BrowserIdentity>,
        restart_on_process_read: Option<usize>,
        process_reads: Cell<usize>,
        effects: RefCell<Vec<usize>>,
    }

    impl Default for FakeProvider {
        fn default() -> Self {
            Self {
                capability: BrowserMacOSCapability::ExactIdentity,
                live: vec![identity()],
                focused: Ok(0),
                focused_live: None,
                restart_on_process_read: None,
                process_reads: Cell::new(0),
                effects: RefCell::new(Vec::new()),
            }
        }
    }

    impl BrowserIdentityProvider for FakeProvider {
        type Tab = Handle;

        fn capability(&self) -> BrowserMacOSCapability {
            self.capability
        }

        fn process_started_at(&self, pid: i32) -> Result<String, String> {
            assert_eq!(pid, 77);
            let read = self.process_reads.get() + 1;
            self.process_reads.set(read);
            if self.restart_on_process_read.is_some_and(|at| read >= at) {
                Ok(format!("{:016x}", 1_791_000_001.5f64.to_bits()))
            } else {
                Ok(identity().process_started_at)
            }
        }

        fn tabs(&self, pid: i32) -> Result<Vec<Self::Tab>, String> {
            assert_eq!(pid, 77);
            Ok((0..self.live.len()).map(Handle).collect())
        }

        fn evidence(&self, tab: &Self::Tab) -> Result<BrowserIdentity, String> {
            if !self.effects.borrow().is_empty()
                && tab.0 == self.focused.clone().unwrap_or(usize::MAX)
            {
                if let Some(identity) = &self.focused_live {
                    return Ok(identity.clone());
                }
            }
            self.live
                .get(tab.0)
                .cloned()
                .ok_or_else(|| "stale retained tab handle".into())
        }

        fn focus_tab(&self, tab: &Self::Tab) -> Result<(), String> {
            self.effects.borrow_mut().push(tab.0);
            Ok(())
        }

        fn globally_focused_tab(&self, pid: i32) -> Result<Self::Tab, String> {
            assert_eq!(pid, 77);
            self.focused.clone().map(Handle)
        }
    }

    fn assert_refused(receipt: &BrowserRestoreReceipt, provider: &FakeProvider) {
        assert!(matches!(
            receipt.dispatch,
            DispatchResult::NotDispatched { .. }
        ));
        assert!(matches!(
            receipt.verification,
            VerificationResult::Unverified { .. }
        ));
        assert!(provider.effects.borrow().is_empty());
    }

    #[test]
    fn exact_success_retains_handle_and_verifies_complete_postcondition() {
        let provider = FakeProvider::default();
        let receipt = restore_browser_identity(&identity(), &provider);
        assert_eq!(receipt.dispatch, DispatchResult::RaiseAccepted);
        assert_eq!(receipt.verification, VerificationResult::Verified);
        assert_eq!(receipt.postcondition, Some(identity()));
        assert_eq!(*provider.effects.borrow(), vec![0]);
    }

    #[test]
    fn witnessing_reads_the_globally_focused_exact_tab_identity() {
        let provider = FakeProvider::default();
        assert_eq!(witness_focused_browser(77, &provider).unwrap(), identity());
        assert!(provider.effects.borrow().is_empty());
    }

    #[test]
    fn duplicate_exact_identity_refuses_even_when_other_fields_could_differ() {
        let provider = FakeProvider {
            live: vec![
                identity(),
                BrowserIdentity {
                    url: "https://example.test/another-url".into(),
                    ..identity()
                },
            ],
            ..FakeProvider::default()
        };
        let receipt = restore_browser_identity(&identity(), &provider);
        assert_refused(&receipt, &provider);
        assert!(
            matches!(receipt.dispatch, DispatchResult::NotDispatched { reason } if reason.contains("ambiguous"))
        );
    }

    #[test]
    fn stale_or_restarted_process_refuses_before_dispatch() {
        for restart in [1, 2] {
            let provider = FakeProvider {
                restart_on_process_read: Some(restart),
                ..FakeProvider::default()
            };
            assert_refused(&restore_browser_identity(&identity(), &provider), &provider);
        }
    }

    #[test]
    fn post_dispatch_mismatch_is_failed_and_never_retried() {
        let provider = FakeProvider {
            focused: Ok(1),
            focused_live: Some(BrowserIdentity {
                url: "https://example.test/other".into(),
                ..identity()
            }),
            ..FakeProvider::default()
        };
        let receipt = restore_browser_identity(&identity(), &provider);
        assert_eq!(receipt.dispatch, DispatchResult::RaiseAccepted);
        assert!(matches!(
            receipt.verification,
            VerificationResult::Failed { .. }
        ));
        assert!(receipt.postcondition.is_none());
        assert_eq!(*provider.effects.borrow(), vec![0]);
    }

    #[test]
    fn process_restart_after_dispatch_is_failed_without_retry() {
        let provider = FakeProvider {
            restart_on_process_read: Some(4),
            ..FakeProvider::default()
        };
        let receipt = restore_browser_identity(&identity(), &provider);
        assert_eq!(receipt.dispatch, DispatchResult::RaiseAccepted);
        assert!(matches!(
            receipt.verification,
            VerificationResult::Failed { reason } if reason.contains("lifetime changed")
        ));
        assert_eq!(*provider.effects.borrow(), vec![0]);
    }

    #[test]
    fn unavailable_provider_refuses_without_inspection_or_fallback() {
        let provider = FakeProvider {
            capability: BrowserMacOSCapability::Unsupported,
            ..FakeProvider::default()
        };
        assert_refused(&restore_browser_identity(&identity(), &provider), &provider);
        assert_eq!(provider.process_reads.get(), 0);
    }

    #[test]
    fn mismatch_does_not_use_url_to_select_another_tab() {
        let provider = FakeProvider {
            live: vec![BrowserIdentity {
                url: URL.into(),
                tab_token: "different-tab".into(),
                ..identity()
            }],
            ..FakeProvider::default()
        };
        assert_refused(&restore_browser_identity(&identity(), &provider), &provider);
        assert!(provider.effects.borrow().is_empty());
    }

    #[test]
    fn endpoint_requires_explicit_loopback_and_never_dns() {
        assert!(LocalDevToolsConfig::new("127.0.0.1".parse().unwrap(), 9222, 77).is_ok());
        assert!(LocalDevToolsConfig::new("::1".parse().unwrap(), 9222, 77).is_ok());
        assert!(LocalDevToolsConfig::new("8.8.8.8".parse().unwrap(), 9222, 77).is_err());
        assert!(parse_ws_url("ws://localhost:9222/devtools/browser/x").is_err());
        assert!(parse_ws_url("wss://127.0.0.1:9222/devtools/browser/x").is_err());
    }
}
