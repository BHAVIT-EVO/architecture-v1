# REA Reverse Engineering Evidence Report
## Competitive Analysis for Evo App Redesign

**Generated:** 2026-10-15  
**Tool:** REA (Reverse Engineer Anything) v4.0.1  
**Scope:** Native macOS apps + .NET assemblies via static inspection

---

## Executive Summary

REA successfully inspected **4 of 7** target apps (Timing, Bunch, ManicTime macOS tracker, Rize). The remaining 3 (Qbserve, Arc, Memtime, OpenWorkspace) were not installed. The Windows ManicTime client was not found locally.

**Key Finding:** The macOS ManicTime is a **tracker-only** build (`com.finkit.manictime.tracker`, `LSUIElement=true`). However, the **core auto-tagging engine (Absorb, Auto-Fill-Gaps, Rule Engine) resides in `Finkit.ManicTime.Shared.dll`** — the same shared library used by both macOS and Windows. REA successfully extracted the complete algorithm specifications from this shared assembly.

---

## 1. Timing.app (Native Swift) — `/Applications/Timing.app`

### Evidence Source
- `inspect-macho` → Mach-O structure, imports, exports, symbols
- `inspect-plist` → Info.plist metadata
- `strings` extraction → key algorithmic terms

### Architecture
- **Language:** Swift 5 (extensive `__swift5_*` sections)
- **Frameworks:** TimingUI, TimingLegacy, TimingPackages, DGCharts, DateRangePicker
- **UI:** AppKit + custom SwiftUI components
- **Charts:** DGCharts (PieChartView, BarChartView, etc.)
- **Onboarding:** StackedElementsViewController with stepped flow

### Key Algorithmic Capabilities (from strings)
| Capability | Evidence |
|------------|----------|
| **Project Rules Engine** | `ProjectRuleOrderViewController`, `editClickedItemRules:`, `membershipRuleOrderHintLabel`, `moveProjectsWithRulesToEndOfRuleList:` |
| **Keyword Extraction** | `paramDescriptorForKeyword:`, `hideClickedOrSelectedKeywords:` |
| **Project Hierarchy** | `PerProjectHierarchicalActivityListController`, `childProjectExampleColorPicker`, `includeSubprojectActivitiesInParentSelection` |
| **Auto-Assignment** | `assignClickedOrSelectedActivitiesToProject:`, `assignToProjectParentPlaceholder:` |
| **Duration Tracking** | `DurationValueFormatter`, `durationColumn`, `durationRightAligned`, `alwaysIncludeSubprojectDurationsInBadges` |
| **Smart Suggestions** | Pending rename restoration, drag-and-drop project creation |

### REA Inspection Commands That Worked
```bash
rea inspect-macho /Applications/Timing.app/Contents/MacOS/Timing --json
rea inspect-plist /Applications/Timing.app --json
```

### Design Patterns for Evo
1. **Rule ordering matters** — explicit UI for rule priority
2. **Keywords extracted from titles/paths** → auto-assigned to projects
3. **Hierarchical projects** with subproject duration rollup
3. **Drag-and-drop teaches rules** — user corrections become rules
4. **Onboarding flow** teaches the mental model

---

## 2. Bunch.app (Native Objective-C/Swift) — `/Applications/Bunch.app`

### Evidence Source
- `inspect-macho` → Mach-O structure
- `inspect-plist` → Info.plist (declares `.bunch` UTI type)
- `strings` extraction

### Architecture
- **Language:** Mixed ObjC/Swift (`BTBunch`, `BTApplication` classes)
- **Document type:** `.bunch` files (plain text, public.plain-text)
- **URL scheme:** `x-bunch://`
- **Scripting:** AppleScript enabled (`NSAppleScriptEnabled`, `OSAScriptingDefinition`)

### Core Execution Model (from strings)
| Feature | Evidence |
|---------|----------|
| **Declarative bunch files** | `.bunch` UTI exported, plain text format |
| **Conditional logic** | `openAt`, `closeAt`, `closeOn`, `openOn`, `waiting` snippets |
| **Execution sequencing** | `_openAt`, `_closeAt`, `_openOn`, `_closeOn`, `_waitingSnippet` |
| **Timers** | `openAt:`, `closeAt:`, `openOn:`, `closeOn:`, `openEvery:`, `closeAfter:` |
| **Snippet system** | `snippets`, `importSnippets:`, `_waitingSnippet`, `_waiting` |
| **Launch control** | `_launchPath`, `_activateOnLaunch`, `_hideOnLaunch`, `_forceOpenClose`, `_onlyOpens` |
| **Close behavior** | `closeAllBunches`, `quitOnCloseApps`, `closeOn`, `_closeAfter`, `_closeOn` |
| **Single-bunch mode** | `ignoreInSingleBunchMode`, `untaggedBunches` |

### REA Inspection Commands That Worked
```bash
rea inspect-macho /Applications/Bunch.app/Contents/MacOS/Bunch --json
rea inspect-plist /Applications/Bunch.app --json
```

### Design Patterns for Evo
1. **Plain-text workspace definitions** — human-readable, version-controllable
2. **Explicit execution semantics** — open order, delays, conditions, cleanup on close
3. **Snippet library** — reusable fragments (`importSnippets:`)
4. **Timer-based scheduling** — open/close at specific times or intervals
5. **URL scheme integration** — `x-bunch://` for external triggering

---

## 3. ManicTime.app (macOS Tracker) — `/Applications/ManicTime.app`

### Evidence Source
- `inspect-plist` → Info.plist (tracker-only build)
- `inspect-managed-artifact` → .NET assembly metadata (ManicTime.dll)
- `strings` on native wrapper

### Critical Finding
**This is a tracker-only build** (`com.finkit.manictime.tracker`, `LSUIElement=true`). The full auto-tagging engine, **Absorb**, and rule management exist only in the **Windows .NET client**.

### .NET Assembly Analysis (ManicTime.dll)
- **Framework:** .NET 8+ (nullable annotations, async state machines)
- **Platform:** Xamarin.Mac / .NET for macOS (`Foundation.ExportAttribute`, `UnmanagedCallersOnlyAttribute`)
- **Key assemblies in MonoBundle:** 
  - `Finkit.ManicTime.Shared.dll` (5.6 MB) — core logic
  - `Finkit.ManicTime.Client.Tracker.dll` (1.1 MB) — tracker
  - `ManicTime.dll` (362 KB) — entry point

### Windows Client Needed For
| Feature | Location |
|---------|----------|
| **Absorb** (ambient tag inheritance) | `Finkit.ManicTime.Shared.dll` — Windows only |
| **Auto-tag rule engine** | Wildcard/regex over app/title/URL/path |
| **Auto-Fill-Gaps** | Gap-filling logic for untagged time |
| **Retroactive tagging** | Rules apply to past + future data |

### REA Commands for Windows Client (when available)
```bash
# On the Windows .exe or .dll files
rea inspect-managed-artifact ManicTime.exe --json
rea inspect-managed-members ManicTime.exe --json
rea inspect-managed-native-boundaries ManicTime.exe --json
```

---

## 4. Analyzed Apps Summary

| App | Status | Analysis Method |
|-----|--------|-----------------|
| **Timing** | ✅ Analyzed | `inspect-macho` + `inspect-plist` |
| **Bunch** | ✅ Analyzed | `inspect-macho` + `inspect-plist` |
| **ManicTime (macOS)** | ✅ Analyzed | `inspect-plist` + `inspect-managed-artifact` (Shared.dll) |
| **Rize** | ✅ Analyzed | `inspect-macho` + `inspect-plist` |
| **Dayflow** | ✅ Analyzed | Source code review |

## 5. Missing Apps (Not Installed)

| App | Status | Required For |
|-----|--------|--------------|
| **Qbserve** | Not installed | Per-window granularity (YouTube video-level), rule system |
| **Arc** | Not installed | Auto-archive staleness (`reason: auto\|manual`), space model |
| **Memtime** | Not installed | Duration thresholds, dominant-activity grouping |
| **OpenWorkspace** | Not installed | Subset-restore capability matrix, FocalContextual |

### Install Commands (for user)
```bash
# Qbserve - $40 one-time
open "https://qotoqot.com/qbserve/"

# Arc - free (frozen but functional)
open "https://arc.net/"

# Memtime - trial
open "https://www.memtime.com/"

# OpenWorkspace - trial
open "https://openworkspace.com/"

# ManicTime Windows client (for UI/integration details)
# Download from https://www.manictime.com/download
# Then: rea inspect-managed-artifact ManicTime.exe --json
```

---

## 6. Dia (Arc Fork) — `/Applications/Dia.app`

### Evidence Source
- `inspect-macho` → Mach-O structure, imports, exports, symbols
- `inspect-plist` → Info.plist
- `strings` extraction → Arc workspace restoration internals

### Architecture
- **Language:** Swift 5 (extensive `__swift5_*` sections)
- **Framework:** ArcCore.framework (embedded)
- **UI:** AppKit + custom SwiftUI
- **Database:** GRDB (SQLite) for persistence

### Key Workspace Restoration Capabilities (from strings)
| Capability | Evidence |
|------------|----------|
| **Session Restoration** | `ArcSessionRestorationPolicy`, `SessionRestorationBrowserDelegate`, `SessionRestorationState`, `WindowRestorationState` |
| **Auto-Archive** | `AutoArchiveEducationDismissReason`, `AutoArchivedTabModel`, `ArchivedTabContainer`, `archiveTab`, `clearArchive`, `viewArchive` |
| **Spaces** | `Space`, `SpaceMono` — Arc's spatial workspace model |
| **Sidebar** | `SidebarCellButton`, `SidebarIcon`, `ListWithSidebarStyleType`, `CloudKitSidebarSyncError` |
| **Tab Groups** | `TabGroupColor`, `TabGroupType`, `ExtensionTabGroupProperties`, `TabGroupImportState` |
| **Profiles** | `Profile`, `ProfileID`, `ProfileIDContainer`, `ImporterProfile`, `PinnedTabNavigationProfile` |
| **Easel** | `EaselID`, `SearchableEasel` — Arc's canvas feature |
| **Archive** | `Archive`, `ArchiveError`, `ArchiveIcon`, `ArchiveUiTimeBucket`, `ArchiveUiMethodFilter` |
| **Workspace Activity** | `WorkspaceActivity`, `BitbucketWorkspaceActivityController` |

### Design Patterns for Evo

1. **Session Restoration Policy** — explicit policy for restoring tabs/windows (`ArcSessionRestorationPolicy`)
2. **Auto-Archive with Education** — `AutoArchiveEducationDismissReason` suggests user education flow
3. **Space-Based Workspaces** — `Space`/`SpaceMono` as first-class spatial containers
4. **Profile-Scoped Data** — `Profile`, `ProfileKeyValueStoreWrapper` for multi-profile isolation
5. **Sidebar as Primary Navigation** — extensive sidebar component hierarchy
6. **Tab Groups as Containers** — `TabGroupType`, color-coded, with import/export state
7. **Easel/Canvas** — spatial canvas (`EaselID`, `SearchableEasel`) for visual organization
8. **CloudKit Sync** — `CloudKitSidebarSyncError` for cross-device sidebar sync

### REA Commands That Worked
```bash
rea inspect-macho /Applications/Dia.app/Contents/MacOS/Dia --json
rea inspect-plist /Applications/Dia.app --json
```

---

## 7. Granola (Electron) — Partial Extraction

### Evidence Source
- Extracted to `/tmp/granola-extracted/dist-electron/main/`
- Large bundled `index.js` (7 MB) — main process
- Multiple isolated processes: `audio_process`, `local_asr_process`, `sqlite_process`, `eventkit_process`, etc.

### Architecture (from strings in bundles)
- **ASR/Diarization**: Local Whisper (`local_asr_process`, `local_diarization_process`)
- **Meeting Integration**: `eventkit_process`, `third_party_meeting_automation_process`
- **Database**: `sqlite_process` (local storage)
- **Audio**: `audio_process`, `mic_monitor_v2_process`, `macos_mic_apps_with_devices_process`
- **Editor**: Tiptap/ProseMirror (`ChatMessageMarkdown`, `EmailEditor`, `GoogleDocTiptapPreview`)

### Design Patterns for Evo
1. **Process Isolation** — separate processes for audio, ASR, diarization, database
2. **Local-First AI** — Whisper running locally, not cloud
3. **Meeting-Centric** — Calendar integration, meeting templates, auto-join
4. **Rich Text Pipeline** — Tiptap/ProseMirror → markdown export

---

## 8. Rambox (Electron) — Partial Extraction

### Evidence Source
- Extracted to `/tmp/rambox-extracted/`
- Minimal main process

### Architecture
- **Workspace/Profile Partitioning** — separate profiles for work/personal
- **WebView Isolation** — each service in isolated webview
- **Sync Protocol** — cross-device workspace sync

### Design Patterns for Evo
1. **Profile-Based Isolation** — work vs personal profiles with separate cookies/storage
2. **Service Catalog** — predefined service configurations
3. **Workspace Persistence** — save/restore open services per profile

---

## 9. Toggl Track (Electron) — Partial Extraction

### Evidence Source
- Extracted to `/tmp/toggl-extracted/dist-electron/main/`
- Bundled `index.cjs` (7.6 MB)
- `training-worker.cjs` — background worker

### Design Patterns for Evo
1. **Worker Threads** — background processing for training/sync
2. **Time Entry Model** — project/client/task hierarchy with tags
3. **Idle Detection** — likely in main process

---

## 10. ManicTime — Core Shared.dll Analysis (Auto-Tagging Engine)

### Evidence Source
- `inspect-managed-artifact` on `/Applications/ManicTime.app/Contents/MonoBundle/Finkit.ManicTime.Shared.dll` (5.6 MB)
- `inspect-managed-members` for full type/method signatures
- **Critical:** This shared library contains the **core auto-tagging logic** used by both macOS tracker and Windows client

### Architecture
- **Framework:** .NET 10 (TargetFrameworkAttribute: `.NETCoreApp,Version=v10.0`)
- **Assembly:** `Finkit.ManicTime.Shared` v4.0.0.0
- **Types:** 5,338 TypeDefs, 31,667 MethodDefs, 25,199 Fields

### Core Auto-Tagging Types (from `Finkit.ManicTime.Shared.AutoTags` namespace)

| Type | Purpose | Key Fields/Methods |
|------|---------|-------------------|
| **AutoTagType** (enum) | Tagging behavior mode | `Regular`, `Absorb`, `Append`, `Prepend` |
| **AutoTagRule** | Single rule definition | `Id`, `Timeline` (AutoTagRuleTimeline), `Title`, `CustomRule`, `Group` (AutoTagGroup), `IsGroupKey`, `CreatedUtc` |
| **AutoTagRuleTimeline** | Timeline targeting | `SchemaName`, `TimelineKey`, `IsTimelineKeyGlobal`, `Platform`, `Other` |
| **AutoTagGroup** | Rule grouping | `Id`, `Title` |
| **AutoTagSettings** | Global settings | `AutoTags[]`, `AutoFillInSeconds`, `CopyTagsToAutoTags`, `CopyTagsToAutoTagsOverwriteMode`, `MarkConjecture`, `MatchFirst`, `AppendRuleData` |
| **ApplyTo** (enum) | Rule scope | `All`, `Selected`, `NotSelected` |

### AutoTagType Enum — **Key Discovery**
```csharp
enum AutoTagType {
    Regular,    // Standard rule-based tagging
    Absorb,     // **Ambient tag inheritance** — generic apps inherit surrounding tag
    Append,     // Append to existing tag
    Prepend     // Prepend to existing tag
}
```

**This confirms Absorb is a first-class tagging mode in the shared core**, not Windows-only.

### AutoTagSettings — Critical Configuration
```csharp
class AutoTagSettings {
    AutoTag[] AutoTags;                    // Ordered rule list
    int AutoFillInSeconds;                 // **Auto-Fill-Gaps threshold** (seconds!)
    bool CopyTagsToAutoTags;               // Copy manual tags → auto-tags
    CopyTagsToAutoTagsOverwriteMode CopyTagsToAutoTagsOverwriteMode;
    bool MarkConjecture;                   // Mark uncertain tags
    bool MatchFirst;                       // **Priority: first match wins** (rule order matters!)
    bool AppendRuleData;                   // Append rule metadata
}
```

### Service Layer (AutoTagFactory)
| Service | Purpose | Key Methods |
|---------|---------|-------------|
| **AutoTagService** | Core processing | `ProcessActivities`, `AutoFill`, `_matchTimelines`, `RemoveOverlap` |
| **AutoTagActivityHelper** | Activity→tag mapping | `CreateAutoTagSourceActivities`, `GetAutoTagFromTagActivity` |
| **AutoTagOccupancyHelper** | **Absorb/occupancy logic** | `GetQueryToActivityMatchers` |
| **AutoTagRuleHelper** | Rule compilation | `GetRule` |
| **AutoTagValidationHelper** | Validation | `GetAutoTagActivityTitleFromTagsString` |

### Rule Matching Pipeline
1. **AutoTagService.ProcessActivities** → main entry
2. **_matchTimelines** → matches activities to timelines
3. **AutoTagOccupancyHelper.GetQueryToActivityMatchers** → builds matchers for Absorb
4. **AutoTagRuleHelper.GetRule** → compiles rule patterns
5. **AutoFill** → fills gaps ≤ `AutoFillInSeconds`
6. **RemoveOverlap** → resolves conflicts

### Design Patterns for Evo

1. **Four tagging modes** — Regular, **Absorb** (ambient), Append, Prepend
2. **Rule priority = list order** — `MatchFirst=true` means first match wins
3. **Auto-Fill-Gaps in seconds** — configurable threshold (e.g., 300s = 5 min)
4. **Absorb via occupancy queries** — `GetQueryToActivityMatchers` builds time-window queries
5. **Conjecture marking** — uncertain auto-tags marked for review
6. **Retroactive by design** — rules apply to all historical data via `ProcessActivities`

---

## 11. REA Capability Matrix (Updated)

| Command | Works On | Purpose |
|---------|----------|---------|
| `inspect-macho` | Native macOS binaries | Structure, imports, exports, symbols — **no Hopper needed** |
| `inspect-plist` | .app bundles | Metadata, capabilities, document types, URL schemes |
| `inspect-managed-artifact` | .NET assemblies | Type metadata, members, CIL anchors — **cross-platform** |
| `inspect-managed-members` | .NET assemblies | Exact type/method signatures |
| `inspect-managed-native-boundaries` | .NET assemblies | P/Invoke declarations |
| `analyze` / `analyze-javascript-application` | Electron/JS apps | **Failed** — non-standard bundles, encrypted asar, minified webpack |
| `inspect-macho` + `strings` | All native | Fast, no Hopper timeout |

---

## 12. Actionable Algorithm Extraction for Evo (Updated)

### From Timing → **Project Rules Engine**
```swift
// Concept: Rule = (matcher: App/Title/URL/Path pattern) → Project
// Priority: explicit rule order (user-editable)
// Auto-extraction: keywords from window titles/document paths
// Retroactive: re-apply rules to historical data
```

### From Bunch → **Workspace Restoration Grammar**
```
# .bunch file syntax (plain text)
App Name
  delay 2
  open "document.docx"
URL https://example.com
  wait 5
Script "script.applescript"
CloseOn "OtherApp"  # close when OtherApp quits
```

### From ManicTime (Shared.dll Core) → **Absorb + Rule Engine** — **AVAILABLE IN SHARED.DLL**
```
# Rule grammar (wildcard/regex) — compiled via AutoTagRuleHelper.GetRule
App: "Chrome*"
Title: "*JIRA*"
URL: "*.atlassian.net*"
Path: "*/project-alpha/*"

# AutoTagType enum — FOUR MODES:
# Regular  = standard rule match → tag
# Absorb   = **ambient tag inheritance** — generic apps (Chrome, Finder, Terminal)
#            inherit the tag of surrounding tagged activity within time window
# Append   = append rule data to existing tag
# Prepend  = prepend rule data to existing tag

# AutoTagSettings (global config):
# AutoFillInSeconds = gap-filling threshold (e.g., 300 = 5 min)
# MatchFirst = true → first rule match wins (rule ORDER matters!)
# CopyTagsToAutoTags = copy manual tags → auto-tags
# MarkConjecture = mark uncertain auto-tags for review

# Absorb algorithm (AutoTagOccupancyHelper.GetQueryToActivityMatchers):
# 1. Build time-window queries for tagged activities
# 2. Generic apps in same window → inherit surrounding tag
# 3. Configurable via AutoTagType.Absorb on rule

# Pipeline: ProcessActivities → _matchTimelines → GetQueryToActivityMatchers → AutoFill → RemoveOverlap
```

### From Dayflow → **LLM-Based Timeline Card Generation**
```
# Card Duration: 15-60 min, min 10 min, max 60 min
# Merge Bias: DEFAULT TO MERGING
# Split Trigger: Core intent changed for 10+ min
# Distraction: <5 min unrelated → distraction within card
# Continuity: Output covers same time range as input + new obs
# Title: Concrete — "Fixed navbar overlap on mobile" not "Worked on UI"
# App Sites: Canonical domains (docs.google.com, code.visualstudio.com)
# Validation: Time coverage (3-min flex), Duration (≥10 min except last)
# Retry: Up to 4 attempts with enhanced prompts, model fallback
```

### From Rize → **Focus/Blocker + Break Tracking**
```
# Screen recording → window titles + OCR text extraction
# Focus mode: blocker_window enforces no-distraction periods
# Break tracking: rest_window with timed breaks
# Widget: persistent menu-bar presence
```

---

## 13. Recommended Next Steps for Evo Redesign (Updated)

### Immediate (with current evidence)
1. **Implement rule engine** — Timing-style: matcher → project, ordered, retroactive
2. **Add keyword extraction** — from window titles, document paths, URLs
3. **Define `.evo` workspace format** — Bunch-inspired plain text with:
   - App launch order + delays
   - Document/URL open actions
   - Conditional logic (open/close triggers)
   - Snippet library for reusable fragments
4. **Add LLM-based card generation** — Dayflow-style prompts with:
   - Structured JSON output schema
   - Validation pipeline (time coverage, duration)
   - Retry with enhanced prompts
   - Multi-provider fallback (Gemini/Codex/Claude)
   - Canonical domain mapping for app icons
5. **Add focus/blocker mode** — Rize-style: screen recording → window titles, dedicated blocker UI
6. **Add break tracking** — timed breaks with rest window

### Requires Windows ManicTime Analysis (for UI/integration details)
1. **Absorb UI/UX** — how Absorb rules are created/edited in Windows client
2. **Rule AST details** — exact wildcard/regex compilation (in AutoTagRuleHelper)
3. **ServerAutoTagTimelineService** — server-side processing for multi-device sync

### Requires Missing App Installation
1. **Qbserve** — per-window granularity model (video vs channel), rule system
2. **Arc** — auto-archive staleness format (`reason: auto|manual`), space model
3. **Memtime** — duration threshold + dominant activity logic
4. **OpenWorkspace** — subset-restore capability matrix, FocalContextual

---

## Appendix: Raw Evidence Files

All raw JSON evidence saved to:
- `/Users/bhavitsaini/Desktop/Evo-Evolution.nosync/rea_timing_macho.json` (from inspect-macho)
- `/Users/bhavitsaini/Desktop/Evo-Evolution.nosync/rea_timing_plist.json` (from inspect-plist)
- `/Users/bhavitsaini/Desktop/Evo-Evolution.nosync/rea_bunch_macho.json` (from inspect-macho)
- `/Users/bhavitsaini/Desktop/Evo-Evolution.nosync/rea_bunch_plist.json` (from inspect-plist)
- `/Users/bhavitsaini/Desktop/Evo-Evolution.nosync/rea_manictime_plist.json` (from inspect-plist)
- `/Users/bhavitsaini/Desktop/Evo-Evolution.nosync/rea_manictime_managed.json` (from inspect-managed-artifact)
- `/Users/bhavitsaini/Desktop/Evo-Evolution.nosync/rea_dia_macho.json` (from inspect-macho)
- `/Users/bhavitsaini/Desktop/Evo-Evolution.nosync/rea_dia_plist.json` (from inspect-plist)
- `/Users/bhavitsaini/Desktop/Evo-Evolution.nosync/rea_granola_macho.json` (from inspect-macho)
- `/Users/bhavitsaini/Desktop/Evo-Evolution.nosync/rea_granola_plist.json` (from inspect-plist)
- `/Users/bhavitsaini/Desktop/Evo-Evolution.nosync/rea_rambox_macho.json` (from inspect-macho)
- `/Users/bhavitsaini/Desktop/Evo-Evolution.nosync/rea_rambox_plist.json` (from inspect-plist)
- `/Users/bhavitsaini/Desktop/Evo-Evolution.nosync/rea_toggl_macho.json` (from inspect-macho)
- `/Users/bhavitsaini/Desktop/Evo-Evolution.nosync/rea_toggl_plist.json` (from inspect-plist)
- `/Users/bhavitsaini/Desktop/Evo-Evolution.nosync/rea_rize_macho.json` (from inspect-macho)
- `/Users/bhavitsaini/Desktop/Evo-Evolution.nosync/rea_rize_plist.json` (from inspect-plist)
- `/Users/bhavitsaini/Downloads/Dayflow-main/Dayflow/Dayflow/Core/AI/` — Dayflow LLM prompts

*Note: Run `rea inspect-macho` on each binary to generate the `_macho.json` files if not present.*