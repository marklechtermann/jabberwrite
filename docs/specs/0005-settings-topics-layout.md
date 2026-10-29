---
status: in-progress  # draft | accepted | in-progress | done
adrs: []       # e.g. [0001]
---

# 0005: Settings window with topics

## Goal

The settings window is one long scrolling page (Permissions, Shortcut, History, Files, Updates, About). Split it into topics: a navigation on the left, the selected topic on the right. Each topic stays short, and future options (specs 0003, ideas such as language or microphone) have an obvious place.

## Scope

**Layout**

- Header across the full width: app title and status line. A config error (`config_error`) is shown here so it is visible in every topic.
- Left: navigation, about 150 px wide, one button per topic, the active one highlighted. Right: the content of the active topic, scrolling on its own.
- Window size 760×560 with a minimum size; still resizable.
- Plain React state for the active topic. No router, no new dependency.

**Topics**

| Topic | Content |
| --- | --- |
| General | Shortcut. Default topic. |
| History | "Clear when quitting" checkbox, "Clear history" button, list using the full height, history file path. |
| Permissions | Microphone, Input Monitoring, Accessibility. Only shown when `permissions` is available (macOS). A warning marker in the navigation while a permission is missing; the window opens on this topic in that case. |
| Updates | Unchanged content of the current "Updates" section. |
| About | Version, repository link, credits and licenses, config file and log folder paths. |

- Existing sections move as they are; behavior and backend commands do not change.
- Navigation is made of buttons, usable with Tab and Enter.

## Out of scope

New settings, a router, animations, remembering the last topic across restarts, search in settings. Launch at login (spec 0003) and other new options add themselves to General when implemented.

## Tasks

- [x] Layout: header, left navigation, content area (`settings.css`, `Settings()` in `src/settings/main.tsx`)
- [x] Move existing sections into the topics above
- [x] Open on Permissions when one is missing, warning marker in the navigation
- [x] `tauri.conf.json`: window size and minimum size
- [ ] Check light and dark mode and a narrow window

## Done when

- The settings window shows the navigation on the left and one topic at a time on the right; all former sections are reachable and work as before.
- On macOS with a missing permission, the window opens on Permissions and the navigation shows a marker.
- A config error is visible in every topic.
- No new dependencies were added.
