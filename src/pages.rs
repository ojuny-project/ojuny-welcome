#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageKind {
    Options,
    Style,
    Hardware,
    Sources,
}

#[derive(Debug, Clone, Copy)]
pub struct ControlSpec {
    pub path: &'static str,
    pub group: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct PageSpec {
    pub id: &'static str,
    pub title: &'static str,
    pub subtitle: &'static str,
    pub icon: &'static str,
    pub kind: PageKind,
    pub controls: &'static [ControlSpec],
}

const fn control(path: &'static str, group: &'static str) -> ControlSpec {
    ControlSpec { path, group }
}

pub static PAGES: &[PageSpec] = &[
    PageSpec {
        id: "monitors",
        title: "Monitors",
        subtitle: "Arrange your displays and set their position.",
        icon: "preferences-desktop-display-symbolic",
        kind: PageKind::Options,
        controls: &[control("services.nix-settings-gui.monitors", "Monitors")],
    },
    PageSpec {
        id: "audio",
        title: "Audio",
        subtitle: "Choose the PipeWire server settings.",
        icon: "audio-card-symbolic",
        kind: PageKind::Options,
        controls: &[
            control("ojuny.audio.enable", "Audio"),
            control("ojuny.audio.sampleRate", "Audio"),
            control("ojuny.audio.quantum", "Audio"),
            control("ojuny.audio.jack", "Audio"),
        ],
    },
    PageSpec {
        id: "browser",
        title: "Browser",
        subtitle: "Pick the default web browser.",
        icon: "web-browser-symbolic",
        kind: PageKind::Options,
        controls: &[control("ojuny.browser.selected", "Browser")],
    },
    PageSpec {
        id: "layout",
        title: "Layout",
        subtitle: "Choose the default window layout.",
        icon: "view-grid-symbolic",
        kind: PageKind::Options,
        controls: &[control("ojuny.workspaces.defaultLayout", "Workspaces")],
    },
    PageSpec {
        id: "style",
        title: "UI config",
        subtitle: "Pick a starting bar and dock layout.",
        icon: "preferences-desktop-theme-symbolic",
        kind: PageKind::Style,
        controls: &[
            control("ojuny.dux.barPreset", "Bar"),
            control("desktop.ojuny.dux.dockConfigs.dock.enabled", "Dock"),
            control("desktop.ojuny.dux.settings.launcherStyle", "Launcher"),
            control("desktop.ojuny.dux.settings.dankLauncherV2Size", "Launcher"),
            control("ojuny.menu.toggleTarget", "Menu"),
        ],
    },
    PageSpec {
        id: "appearance",
        title: "Appearance",
        subtitle: "Choose the palette, fonts, and icon theme.",
        icon: "preferences-desktop-font-symbolic",
        kind: PageKind::Options,
        controls: &[
            control("stylix.polarity", "Theme"),
            control("ojuny.style.palette", "Theme"),
            control("stylix.fonts.sansSerif.name", "Fonts"),
            control("stylix.fonts.monospace.name", "Fonts"),
            control("stylix.icons.dark", "Icons"),
            control("stylix.icons.light", "Icons"),
        ],
    },
    PageSpec {
        id: "hardware",
        title: "Hardware",
        subtitle: "Confirm the detected profile or choose one manually.",
        icon: "computer-symbolic",
        kind: PageKind::Hardware,
        controls: &[
            control("ojuny.hardware.profile", "Hardware profile"),
            control("ojuny.gpu", "Graphics override"),
        ],
    },
    PageSpec {
        id: "boot",
        title: "Boot",
        subtitle: "Configure multiboot support and the recovery UKI.",
        icon: "system-reboot-symbolic",
        kind: PageKind::Options,
        controls: &[
            control("ojuny.multiboot.enable", "Boot entries"),
            control("ojuny.recovery.enable", "Boot entries"),
        ],
    },
    PageSpec {
        id: "apps",
        title: "Applications",
        subtitle: "Enable the applications you want on first boot.",
        icon: "system-software-install-symbolic",
        kind: PageKind::Options,
        controls: &[
            control("programs.steam.enable", "Applications"),
            control("ojuny.apps.vesktop.enable", "Applications"),
            control("ojuny.apps.office.enable", "Applications"),
            control("ojuny.apps.office.craftAll", "Applications"),
            control("ojuny.apps.obsidian.enable", "Applications"),
            control("ojuny.apps.multimedia.obs", "Applications"),
            control("ojuny.apps.multimedia.gimp", "Applications"),
            control("ojuny.apps.multimedia.kdenlive", "Applications"),
        ],
    },
    PageSpec {
        id: "sources",
        title: "Sources",
        subtitle: "Pick the nixpkgs channel and how Ojuny tools are built.",
        icon: "folder-download-symbolic",
        kind: PageKind::Sources,
        controls: &[
            control("ojuny.binaries.mode", "Tool sources"),
            control("ojuny.manual.source", "Manual"),
        ],
    },
    PageSpec {
        id: "nix",
        title: "Nix settings",
        subtitle: "Set the first-boot Nix defaults.",
        icon: "nix-snowflake-symbolic",
        kind: PageKind::Options,
        controls: &[
            control("nixpkgs.config.allowUnfree", "Packages"),
            control("nix.settings.auto-optimise-store", "Store"),
            control("nix.gc.automatic", "Garbage collection"),
            control("nix.settings.max-jobs", "Build"),
        ],
    },
    PageSpec {
        id: "compatibility",
        title: "App compatibility",
        subtitle: "Enable AppImage and Windows file support.",
        icon: "application-x-executable",
        kind: PageKind::Options,
        controls: &[
            control("ojuny.appimage.enable", "AppImage"),
            control("ojuny.raven.enable", "Windows applications"),
        ],
    },
    PageSpec {
        id: "patches",
        title: "Patches",
        subtitle: "Enable the patches included with the template.",
        icon: "applications-system-symbolic",
        kind: PageKind::Options,
        controls: &[
            control("ojuny.patches", "Patches engine"),
            control("ojuny.patch.development.enable", "Patches"),
            control("ojuny.patch.docker.enable", "Patches"),
            control("ojuny.patch.flatpak.enable", "Patches"),
            control("ojuny.patch.nixshell.enable", "Patches"),
            control("ojuny.patch.remoteDesktop.enable", "Patches"),
            control("ojuny.patch.virt.enable", "Patches"),
        ],
    },
];

pub fn page(id: &str) -> Option<&'static PageSpec> {
    PAGES.iter().find(|page| page.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn page_ids_are_unique() {
        let mut ids = HashSet::new();
        for page in PAGES {
            assert!(ids.insert(page.id), "duplicate page id: {}", page.id);
            assert!(!page.controls.is_empty(), "page has no controls: {}", page.id);
        }
    }

    #[test]
    fn page_lookup_finds_every_page() {
        for page in PAGES {
            assert!(self::page(page.id).is_some());
        }
    }

    #[test]
    fn monitors_control_uses_the_declared_option() {
        let control = &page("monitors").unwrap().controls[0];
        assert_eq!(
            control.path, "services.nix-settings-gui.monitors",
            "the monitors widget is bound by nix-settings-gui/docs/comment-syntax.md:385"
        );
    }

    #[test]
    fn control_paths_are_not_generated_values() {
        for page in PAGES {
            for control in page.controls {
                assert!(
                    !control.path.starts_with("desktop.ojuny.mango.rules.")
                        && !control.path.starts_with("ojuny.mango."),
                    "{} is a generated value, not an option: {}",
                    control.path,
                    page.id
                );
            }
        }
    }
}
