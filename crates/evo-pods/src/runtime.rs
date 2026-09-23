//! The pod runtime: the coherent state machine above a [`PodSurface`].
//!
//! Its vocabulary is entirely decisions: plans, recipes, leases, notes.
//! Every side effect goes through the injected surface, so the same code is
//! exercised by the host's macOS surface and by tests' fake surfaces.
//!
//! The runtime owns no grouping truth: pods arrive already-derived from the
//! engine (daemon), recipes are learnings of user gestures, and leases are
//! receipts. Everything else is the platform's.

use crate::config::{ContainMode, PodConfig};
use crate::dim::{self, ActivationPlan};
use crate::lease::{LeaseStep, PodLease};
use crate::pod::{Pod, PodId, PodState};
use crate::policy::InstancePolicy;
use crate::stage::{self, DisplaySignature, StageRecipe};
use crate::suggest::{self, PodSuggestion};
use crate::surface::{CommandSpec, Frame, PodApp, PodSurface, PodWindow};
use crate::wrapper;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub enum PodError {
    /// A pod is active — leave or switch first.
    AlreadyActive { pod: String },
    /// No pod currently active (leave/cycle target).
    NothingActive,
    /// The index was outside the pod list.
    NoSuchPod,
    /// The desktop could not be read at all.
    Surface(String),
    /// Entering a surface-less pod would move and mute the user's desktop
    /// for nothing — refused, same honesty IS-0022 had.
    EmptySurface,
    /// Mothball refused without `force` while the pod owes deltas.
    UnsafeMothball { subjects: Vec<String> },
}

impl std::fmt::Display for PodError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PodError::AlreadyActive { pod } => write!(f, "already in {pod:?} — leave or switch"),
            PodError::NothingActive => write!(f, "no pod is active"),
            PodError::NoSuchPod => write!(f, "no such pod"),
            PodError::Surface(reason) => write!(f, "the desktop could not be read: {reason}"),
            PodError::EmptySurface => {
                write!(f, "this pod has no apps, documents, pages, or titles to contain")
            }
            PodError::UnsafeMothball { subjects } => write!(
                f,
                "mothball would close {} open delta(s): {}",
                subjects.len(),
                subjects.join(", ")
            ),
        }
    }
}

impl std::error::Error for PodError {}

/// What became of a command — a plain note for the bar/presence.
pub type Note = String;

pub struct PodRuntime {
    surface: Box<dyn PodSurface>,
    config: PodConfig,
    /// Engine-derived, daemon-refreshed; the runtime never decides grouping.
    pods: Vec<Pod>,
    active: Option<usize>,
    lease: Option<PodLease>,
    /// Pod id -> recipe for the CURRENT display signature; re-derived on
    /// signature change (learnings from other geometries don't smear).
    recipes: BTreeMap<u64, StageRecipe>,
    current_signature: Option<DisplaySignature>,
    policies: Vec<InstancePolicy>,
    /// Pids never touched (Evo's own processes).
    protected: Vec<i32>,
}

impl PodRuntime {
    pub fn new(surface: Box<dyn PodSurface>, protected: Vec<i32>, config: PodConfig) -> Self {
        Self {
            surface,
            config,
            pods: Vec::new(),
            active: None,
            lease: None,
            recipes: BTreeMap::new(),
            current_signature: None,
            policies: crate::policy::default_policies(),
            protected,
        }
    }

    pub fn config(&self) -> &PodConfig {
        &self.config
    }

    pub fn set_mode(&mut self, mode: ContainMode) {
        self.config.contain = mode;
    }

    pub fn pods(&self) -> &[Pod] {
        &self.pods
    }

    /// Refresh from the daemon's projection. The active pod stays active
    /// by id even if ordering changed; if it vanished, its lease is
    /// replayed so the desktop never drifts.
    pub fn set_pods(&mut self, pods: Vec<Pod>) {
        let active_id = self.active.and_then(|i| self.pods.get(i).map(|p| p.id));
        self.pods = pods;
        self.active = active_id.and_then(|id| self.pods.iter().position(|p| p.id == id));
        if self.active.is_none() && self.lease.is_some() {
            let stale = self.lease.take();
            if let Some(stale) = stale {
                let _ = self.apply_reversal(&stale);
            }
        }
    }

    pub fn active(&self) -> Option<&Pod> {
        self.active.and_then(|i| self.pods.get(i))
    }

    pub fn is_active(&self) -> bool {
        self.lease.is_some()
    }

    pub fn lease(&self) -> Option<&PodLease> {
        self.lease.as_ref()
    }

    pub fn set_policies(&mut self, policies: Vec<InstancePolicy>) {
        self.policies = policies;
    }

    /// Diagnostics: the surface's own verb log, when it keeps one (fake
    /// surfaces in tests do; the live macOS surface keeps nothing).
    pub fn surface_log(&self) -> Vec<String> {
        self.surface.debug_log()
    }

    pub fn recipe(&self, pod_id: PodId) -> Option<&StageRecipe> {
        self.recipes.get(&pod_id.0)
    }

    // ---- inventory -----------------------------------------------------

    fn gather_inventory(&mut self) -> Result<(Vec<PodApp>, Vec<PodWindow>, Vec<Frame>), PodError> {
        let apps = self.surface.running_apps().map_err(PodError::Surface)?;
        let mut windows = Vec::new();
        for app in &apps {
            if self.protected.contains(&app.pid) || !app.regular {
                continue;
            }
            let mut ws = self.surface.windows_of(app.pid).map_err(PodError::Surface)?;
            windows.append(&mut ws);
        }
        let screens = self.surface.screens();
        Ok((apps, windows, screens))
    }

    fn display_signature(&mut self, screens: &[Frame]) -> DisplaySignature {
        let sig = DisplaySignature::from_frames(screens);
        if self.current_signature.as_ref() != Some(&sig) {
            self.current_signature = Some(sig.clone());
        }
        sig
    }

    // ---- verbs -----------------------------------------------------------

    /// Enter pod at `index`: choreograph per recipe, apply containment,
    /// book a lease. Returns the human note.
    pub fn enter(&mut self, index: usize, now_ms: u64) -> Result<Note, PodError> {
        if self.lease.is_some() {
            let pod = self.active().map(|p| p.name.clone()).unwrap_or_default();
            return Err(PodError::AlreadyActive { pod });
        }
        if index >= self.pods.len() {
            return Err(PodError::NoSuchPod);
        }
        if self.pods[index].surfaces.is_empty() {
            return Err(PodError::EmptySurface);
        }
        let (apps, windows, screens) = self.gather_inventory()?;
        let signature = self.display_signature(&screens);

        let pod = &self.pods[index];
        let recipe = {
            let previous = self.recipes.get(&pod.id.0);
            stage::derive_recipe(
                &pod.bundle,
                &pod.companions,
                signature.clone(),
                self.config.satellite_cap,
                self.config.rail_cap,
                previous,
            )
        };
        let plan = dim::plan_activation(
            &pod.surfaces,
            if recipe.roles.is_empty() { None } else { Some(&recipe) },
            &windows,
            &apps,
            &self.protected,
            self.config.contain,
            // The stage is the main screen's usable frame; the surface
            // contract returns screens main-first.
            screens.first(),
        );
        let name = pod.name.clone();
        let id = pod.id;

        let mut lease = PodLease::new(name.clone(), now_ms);
        apply_plan(&mut *self.surface, &plan, &windows, self.config.veil_alpha, &mut lease);

        self.recipes.insert(id.0, recipe);
        self.active = Some(index);
        self.pods[index].state = PodState::Active;

        let note = format!(
            "Entered \"{}\": {} on stage, {} remembered, {} veiled, {} parked, {} apps hidden.",
            lease.pod,
            lease.moved_windows.len(),
            lease.unparked_members.len(),
            lease.veils.len(),
            lease.parked_windows.len(),
            lease.hidden_apps.len()
        );
        self.lease = Some(lease);
        Ok(note)
    }

    /// Leave the active pod: replay the lease backwards, exactly.
    pub fn leave(&mut self) -> Result<Note, PodError> {
        let Some(lease) = self.lease.take() else {
            return Err(PodError::NothingActive);
        };
        self.apply_reversal(&lease)?;
        if let Some(i) = self.active {
            if let Some(pod) = self.pods.get_mut(i) {
                pod.state = PodState::Inactive;
            }
        }
        self.active = None;
        Ok(format!(
            "Left \"{}\" — the desktop is back exactly as it was.",
            lease.pod
        ))
    }

    /// Switch directly: leave whatever is active, then enter the target.
    pub fn switch(&mut self, index: usize, now_ms: u64) -> Result<Note, PodError> {
        if self.lease.is_some() {
            let _ = self.leave();
        }
        self.enter(index, now_ms)
    }

    /// Cycle one step through the pod list (IS-0022's ⌘⇧E habit).
    pub fn cycle(&mut self, now_ms: u64) -> Result<Note, PodError> {
        if self.pods.is_empty() {
            return Err(PodError::NoSuchPod);
        }
        let next = self.active.map(|i| i + 1).unwrap_or(0) % self.pods.len();
        self.switch(next, now_ms)
    }

    fn apply_reversal(&mut self, lease: &PodLease) -> Result<(), PodError> {
        for step in lease.reversal() {
            match step {
                LeaseStep::CloseVeil { handle } => {
                    let _ = self.surface.veil_close(handle);
                }
                LeaseStep::UnhideApp { pid } => {
                    let _ = self.surface.unhide_app(pid);
                }
                LeaseStep::RehideApp { pid } => {
                    let _ = self.surface.hide_app(pid);
                }
                LeaseStep::UnparkWindow { pid, window_id } => {
                    let _ = self.surface.unminimize_window(pid, window_id);
                }
                LeaseStep::ReparkWindow { pid, window_id } => {
                    let _ = self.surface.minimize_window(pid, window_id);
                }
                LeaseStep::MoveWindowBack { pid, window_id, frame } => {
                    let _ = self.surface.move_window(pid, window_id, frame);
                }
            }
        }
        Ok(())
    }

    // ---- learning & gestures ---------------------------------------------

    /// A window of a pod was moved by the person; the recipe learns.
    pub fn learn_frame(&mut self, pod_id: PodId, resource: &str, frame: Frame) {
        if let Some(recipe) = self.recipes.get_mut(&pod_id.0) {
            let screens = self.surface.screens();
            if let Some(screen) = screens.first() {
                recipe.learn(resource, &frame, screen);
            }
        }
    }

    /// Suggestion for a focused loose subject (silence below the floor /
    /// when contested).
    pub fn suggest_for(&self, subject: &str) -> Option<PodSuggestion> {
        let active = self.active.and_then(|i| self.pods.get(i)).map(|p| p.id);
        suggest::suggest(subject, &self.pods, active, &self.config)
    }

    // ---- isolation (P1) ---------------------------------------------------

    /// Stable per-pod profile directory seed (host-friendly), e.g.
    /// `pod-42` — naming by id; legibility comes from the pod bar.
    pub fn profile_dir(pod_id: PodId, root: &str) -> String {
        format!("{}/pod-{}", root.trim_end_matches('/'), pod_id.0)
    }

    /// The launch for an isolated instance of `app_name` bound to this
    /// pod, per the declared InstancePolicy. `None` when no policy
    /// exists: the honest result is "Evo can't isolate this app" — it
    /// never guesses a flag.
    pub fn isolated_launch(
        &self,
        app_name: &str,
        app_path: &str,
        pod_id: PodId,
        profile_root: &str,
    ) -> Option<CommandSpec> {
        let policy = crate::policy::policy_for(&self.policies, app_name)?;
        let mut args = vec!["-na".to_string(), app_path.to_string(), "--args".to_string()];
        args.extend(policy.launch_args(&Self::profile_dir(pod_id, profile_root)));
        Some(CommandSpec {
            program: "/usr/bin/open".into(),
            args,
        })
    }

    /// Runs an isolated launch through the injected surface.
    pub fn launch_isolated(
        &mut self,
        app_name: &str,
        app_path: &str,
        pod_id: PodId,
        profile_root: &str,
    ) -> Option<Note> {
        let spec = self.isolated_launch(app_name, app_path, pod_id, profile_root)?;
        match self.surface.run(spec) {
            Ok(()) => Some(format!("Launched an isolated {app_name} for this pod.")),
            Err(err) => Some(format!("Isolation launch failed: {err}")),
        }
    }

    /// The wrapper spec for giving this pod a Dock/Cmd-Tab identity.
    pub fn wrapper_spec(
        &self,
        pod_id: PodId,
        host_prefix: &str,
        target_binary: &str,
        target_args: Vec<String>,
    ) -> Option<wrapper::WrapperSpec> {
        let pod = self.pods.iter().find(|p| p.id == pod_id)?;
        Some(wrapper::WrapperSpec {
            label: format!("Evo Pod — {}", pod.name),
            bundle_id: wrapper::normalize_bundle_id(host_prefix, &pod.name),
            target_binary: target_binary.to_string(),
            target_args,
        })
    }

    /// Mothball: quit the pod's own apps (recipe kept); refused while the
    /// pod owes unsaved/draft deltas unless forced — charging ahead would
    /// silently eat the only state that lives nowhere.
    pub fn mothball(&mut self, index: usize, force: bool) -> Result<Note, PodError> {
        if index >= self.pods.len() {
            return Err(PodError::NoSuchPod);
        }
        let debt: Vec<String> = self.pods[index]
            .badges
            .iter()
            .filter(|b| {
                matches!(
                    b.kind,
                    crate::pod::BadgeKind::UnsavedEdits | crate::pod::BadgeKind::UnfinishedDraft
                )
            })
            .map(|b| b.subject.clone())
            .collect();
        if !debt.is_empty() && !force {
            return Err(PodError::UnsafeMothball { subjects: debt });
        }
        if self.active == Some(index) {
            let _ = self.leave();
        }
        let (apps, windows, _screens) = self.gather_inventory()?;
        let pod = self.pods[index].clone();
        let mut quits = 0usize;
        for app in &apps {
            if self.protected.contains(&app.pid) {
                continue;
            }
            let owned_by_pod = pod.surfaces.apps.iter().any(|n| n == &app.name)
                || windows
                    .iter()
                    .any(|w| w.pid == app.pid && dim::window_match(w, &pod.surfaces).belongs());
            if owned_by_pod && !app.hidden && self.surface.quit_app(app.pid).is_ok() {
                quits += 1;
            }
        }
        self.pods[index].state = PodState::Mothballed;
        Ok(format!(
            "Mothballed \"{}\": {} apps quit, recipe kept. One key brings it back.",
            pod.name, quits
        ))
    }

    /// Export/import of recipe learnings (user gesture memory).
    pub fn export_recipes(&self) -> String {
        let pairs: Vec<(String, StageRecipe)> = self
            .recipes
            .iter()
            .map(|(id, recipe)| (format!("pod-{id}"), recipe.clone()))
            .collect();
        stage::recipe_store::export(&pairs)
    }

    pub fn import_recipes(&mut self, text: &str) {
        for (pod_label, recipe) in stage::recipe_store::import(text) {
            let Some(id) = pod_label
                .strip_prefix("pod-")
                .and_then(|s| s.parse::<u64>().ok())
            else {
                continue;
            };
            // Learned placements merge into any existing recipe for the
            // same pod & signature; roles are always engine-derived.
            match self.recipes.entry(id) {
                std::collections::btree_map::Entry::Occupied(mut e) => {
                    e.get_mut().placements.extend(recipe.placements);
                }
                std::collections::btree_map::Entry::Vacant(e) => {
                    e.insert(recipe);
                }
            }
        }
    }
}

/// Applies an activation, booking receipts into the lease as it goes.
/// Application order (arrange → veil → park → unbalance apps → activate)
/// and the reversal order in [`PodLease::reversal`] are inverse by
/// construction.
fn apply_plan(
    surface: &mut dyn PodSurface,
    plan: &ActivationPlan,
    inventory: &[PodWindow],
    veil_alpha: f32,
    lease: &mut PodLease,
) {
    // Resurrect first: the space's own minimized windows come back before
    // any choreography, because a minimized window cannot be moved.
    for (pid, window_id) in &plan.unpark_members {
        if surface.unminimize_window(*pid, *window_id).is_ok() {
            lease.unparked_members.push((*pid, *window_id));
        }
    }
    for home in &plan.arrange {
        if let Some(target) = home.frame {
            if let Some(current) = inventory
                .iter()
                .find(|w| w.pid == home.pid && w.window_id == home.window_id)
                .map(|w| w.frame)
            {
                lease.moved_windows.push((home.pid, home.window_id, current));
            }
            let _ = surface.move_window(home.pid, home.window_id, target);
        }
        let _ = surface.raise_window(home.pid, home.window_id);
    }
    for (pid, window_id, frame) in &plan.veil {
        let _ = pid;
        if let Ok(handle) = surface.veil_open(*frame, veil_alpha) {
            lease.veils.push(handle);
        }
    }
    for (pid, window_id) in &plan.park {
        if surface.minimize_window(*pid, *window_id).is_ok() {
            lease.parked_windows.push((*pid, *window_id));
        }
    }
    for pid in &plan.unhide_apps {
        if surface.unhide_app(*pid).is_ok() {
            lease.unhidden_apps.push(*pid);
        }
    }
    for pid in &plan.hide_apps {
        if surface.hide_app(*pid).is_ok() {
            lease.hidden_apps.push(*pid);
        }
    }
    if let Some(pid) = plan.activate {
        let _ = surface.activate_app(pid);
    }
}
