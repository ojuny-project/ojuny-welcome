mod config;
mod pages;

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use adw::prelude::*;
use nix_settings_gui::model::Record;
use nix_settings_gui::widgets::{build, facter_detect, BuildContext, Editor};
use nix_settings_gui::{edit, flake_inputs, nixpkgs, scan};

use config::{NixSettingsGuiConfig, WelcomeConfig};
use pages::{PageKind, PageSpec, PAGES};

fn record_path(record: &Record) -> String {
    let path = record.path();
    path.strip_prefix("config.")
        .unwrap_or(&path)
        .to_string()
}

fn page_icon(name: &str) -> gtk::Image {
    let theme = gtk::gdk::Display::default().map(|display| gtk::IconTheme::for_display(&display));
    let chosen = theme
        .as_ref()
        .and_then(|theme| {
            [name, "preferences-system-symbolic", "application-x-executable-symbolic"]
                .into_iter()
                .find(|candidate| theme.has_icon(candidate))
        })
        .unwrap_or("preferences-system-symbolic");
    gtk::Image::from_icon_name(chosen)
}

const APP_ID: &str = "org.ojuny.Welcome";

struct PageView {
    spec: &'static PageSpec,
    controls: Vec<(Record, Editor)>,
}

struct Wizard {
    flake: String,
    config: RefCell<WelcomeConfig>,
    pages: Vec<PageView>,
    current: Cell<usize>,
    page_stack: gtk::Stack,
    status: gtk::Label,
}

fn main() -> gtk::glib::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let autostart = args.iter().any(|arg| arg == "--autostart");
    let reset = args.iter().any(|arg| arg == "--reset");
    let flake_arg = args
        .iter()
        .position(|arg| arg == "--flake")
        .and_then(|index| args.get(index + 1).cloned());

    let application = adw::Application::builder().application_id(APP_ID).build();
    application.connect_startup(|_| {
        gtk::Window::set_default_icon_name("preferences-system");
    });
    application.connect_activate(move |app| {
        let mut config = WelcomeConfig::load();
        if reset {
            config.reset();
            let _ = config.save();
        }
        if autostart && config.setup_completed {
            std::process::exit(0);
        }
        let flake = flake_arg
            .clone()
            .or_else(|| config.flake_path.clone())
            .or_else(|| std::env::var("OJUNY_FLAKE").ok())
            .unwrap_or_else(|| {
                let home = std::env::var_os("HOME")
                    .map(std::path::PathBuf::from)
                    .unwrap_or_default();
                home.join("repos/nix-flake").display().to_string()
            });
        config.flake_path = Some(flake.clone());
        let _ = config.save();
        build_window(app, flake, config);
    });
    let gtk_args = [std::env::args().next().unwrap_or_default()];
    application.run_with_args(&gtk_args)
}

fn build_window(app: &adw::Application, flake: String, config: WelcomeConfig) {
    let scanned = match scan::scan_root(&flake) {
        Ok(scanned) => scanned,
        Err(error) => {
            eprintln!("ojuny-welcome: cannot scan {flake}: {error}");
            scan::ScanResult::default()
        }
    };
    let mut records = HashMap::new();
    for menu in &scanned.menus {
        for record in &menu.records {
            records.insert(record_path(record), record.clone());
        }
    }
    if let Some(parent) = records.get("desktop.ojuny.dux.dockConfigs.dock").cloned() {
        if let Some(mut enabled) = scan::scan_value(&parent.value)
            .into_iter()
            .find(|record| record.path() == "enabled")
        {
            enabled.file = parent.file.clone();
            enabled.byte_start += parent.byte_start;
            enabled.byte_end += parent.byte_start;
            records.insert(
                "desktop.ojuny.dux.dockConfigs.dock.enabled".to_string(),
                enabled,
            );
        }
    }
    let show_gpu_override = gpu_override_needed(&flake);

    let page_stack = gtk::Stack::builder()
        .transition_type(gtk::StackTransitionType::Crossfade)
        .hexpand(true)
        .vexpand(true)
        .build();
    let mut pages = Vec::new();
    for spec in PAGES {
        let root = gtk::Box::new(gtk::Orientation::Vertical, 12);
        root.set_margin_top(18);
        root.set_margin_bottom(18);
        root.set_margin_start(24);
        root.set_margin_end(24);
        root.set_hexpand(true);
        root.set_vexpand(true);
        root.append(
            &gtk::Label::builder()
                .label(spec.subtitle)
                .xalign(0.0)
                .wrap(true)
                .css_classes(["dim-label"])
                .build(),
        );
        if spec.kind == PageKind::Sources {
            let channels = nixpkgs::channels();
            let labels: Vec<&str> = channels.iter().map(String::as_str).collect();
            let picker = gtk::DropDown::from_strings(&labels);
            let current = current_nixpkgs_channel(&flake).unwrap_or_else(|| "nixos-unstable".to_string());
            if let Some(index) = channels.iter().position(|channel| *channel == current) {
                picker.set_selected(index as u32);
            }
            let heading = gtk::Label::builder()
                .label("Nixpkgs channel")
                .xalign(0.0)
                .css_classes(["title-5"])
                .build();
            let notice = gtk::Label::builder()
                .xalign(0.0)
                .wrap(true)
                .css_classes(["dim-label"])
                .build();
            let choices = channels.clone();
            let target = flake.clone();
            let notice_for = notice.clone();
            picker.connect_selected_notify(move |picker| {
                let Some(channel) = choices.get(picker.selected() as usize) else {
                    return;
                };
                match set_nixpkgs_channel(&target, channel) {
                    Ok(()) => notice_for.set_text(&format!("Set nixpkgs to {channel}")),
                    Err(error) => notice_for.set_text(&format!("Cannot set nixpkgs: {error}")),
                }
            });
            root.append(&heading);
            root.append(&picker);
            root.append(&notice);
        }
        let mut controls = Vec::new();
        let mut last_group = "";
        for control in spec.controls {
            if control.group != last_group {
                last_group = control.group;
                let heading = gtk::Label::builder()
                    .label(control.group)
                    .xalign(0.0)
                    .css_classes(["title-5"])
                    .build();
                root.append(&heading);
            }
            let Some(record) = records.get(control.path) else {
                let missing = gtk::Label::builder()
                    .label(format!("{} is not present in this configuration", control.path))
                    .xalign(0.0)
                    .wrap(true)
                    .css_classes(["dim-label"])
                    .build();
                root.append(&missing);
                continue;
            };
            if control.path == "ojuny.gpu" && !show_gpu_override {
                continue;
            }
            let mut context = BuildContext::default();
            context.base = flake.clone();
            context.records = Rc::new(records.clone());
            let editor = build(record, None, &context);
            let row = gtk::Box::new(gtk::Orientation::Vertical, 4);
            row.set_margin_top(4);
            row.set_margin_bottom(4);
            row.set_hexpand(true);
            row.set_halign(gtk::Align::Fill);
            if let Some(label) = record.spec.label.as_deref() {
                row.append(
                    &gtk::Label::builder()
                        .label(label)
                        .xalign(0.0)
                        .css_classes(["dim-label"])
                        .build(),
                );
            }
            let editor_root = editor.root();
            editor_root.set_hexpand(true);
            editor_root.set_halign(gtk::Align::Fill);
            row.append(&editor_root);
            root.append(&row);
            controls.push((record.clone(), editor));
        }
        if spec.kind == PageKind::Style {
            let presets = ["Windows", "macOS", "Ojuny", "Island"];
            let values = [("taskbar", false), ("status", true), ("default", false), ("simple-island", false)];
            let picker = gtk::DropDown::from_strings(&presets);
            let heading = gtk::Label::builder()
                .label("Starting layout")
                .xalign(0.0)
                .css_classes(["title-5"])
                .build();
            let editors: Vec<(String, Editor)> = controls
                .iter()
                .map(|(record, editor)| (record_path(record), editor.clone()))
                .collect();
            picker.set_selected(current_style_index(&editors) as u32);
            picker.connect_selected_notify(move |picker| {
                let Some((preset, dock)) = values.get(picker.selected() as usize) else {
                    return;
                };
                for (path, editor) in &editors {
                    if path == "ojuny.dux.barPreset" {
                        if let Editor::Dropdown { combo, choices } = editor {
                            if let Some(index) = choices.iter().position(|choice| choice.trim_matches('"') == *preset) {
                                combo.set_selected(index as u32);
                            }
                        }
                    } else if path == "desktop.ojuny.dux.dockConfigs.dock.enabled" {
                        if let Editor::Switch(switch) = editor {
                            switch.set_active(*dock);
                        }
                    }
                }
            });
            root.prepend(&picker);
            root.prepend(&heading);
        }
        let scrolled = gtk::ScrolledWindow::builder()
            .child(&root)
            .vexpand(true)
            .hexpand(true)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .propagate_natural_width(false)
            .build();
        page_stack.add_named(&scrolled, Some(spec.id));
        pages.push(PageView {
            spec,
            controls,
        });
    }

    let (next_steps, reset_row, rebuild_row) = build_next_steps();
    page_stack.add_named(&next_steps, Some("next-steps"));

    let wizard = Rc::new(Wizard {
        flake,
        config: RefCell::new(config.clone()),
        pages,
        current: Cell::new(0),
        page_stack,
        status: gtk::Label::builder()
            .xalign(0.0)
            .css_classes(["dim-label"])
            .build(),
    });

    let sidebar = gtk::ListBox::new();
    sidebar.set_selection_mode(gtk::SelectionMode::Single);
    sidebar.add_css_class("navigation-sidebar");
    for page in &wizard.pages {
        let row = gtk::ListBoxRow::new();
        let content = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        content.append(&page_icon(page.spec.icon));
        content.append(
            &gtk::Label::builder()
                .label(page.spec.title)
                .xalign(0.0)
                .hexpand(true)
                .ellipsize(gtk::pango::EllipsizeMode::End)
                .build(),
        );
        row.set_child(Some(&content));
        row.set_widget_name(page.spec.id);
        sidebar.append(&row);
    }
    let sidebar_scroll = gtk::ScrolledWindow::builder()
        .child(&sidebar)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .build();

    let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    content.set_hexpand(true);
    content.set_vexpand(true);
    content.append(&wizard.page_stack);
    content.append(&wizard.status);

    let paned = gtk::Paned::new(gtk::Orientation::Horizontal);
    paned.set_start_child(Some(&sidebar_scroll));
    paned.set_end_child(Some(&content));
    paned.set_position(230);
    paned.set_hexpand(true);
    paned.set_vexpand(true);
    paned.set_shrink_start_child(true);
    paned.set_shrink_end_child(true);

    let back = gtk::Button::with_label("Back");
    let next = gtk::Button::with_label("Next");
    next.add_css_class("suggested-action");
    let footer = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    footer.set_margin_top(8);
    footer.set_margin_bottom(8);
    footer.set_margin_start(12);
    footer.set_margin_end(12);
    footer.append(&gtk::Label::builder().hexpand(true).build());
    footer.append(&back);
    footer.append(&next);

    let layout = gtk::Box::new(gtk::Orientation::Vertical, 0);
    layout.set_hexpand(true);
    layout.set_vexpand(true);
    layout.append(&paned);

    let header = adw::HeaderBar::new();
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&header);
    toolbar.set_content(Some(&layout));
    toolbar.add_bottom_bar(&footer);
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Welcome to Ojuny")
        .default_width(1180)
        .default_height(760)
        .content(&toolbar)
        .build();

    if wizard.config.borrow().setup_completed {
        show_next_steps(&wizard, &sidebar, &next);
    } else {
        select_page(&wizard, &sidebar, 0);
    }

    {
        let wizard = wizard.clone();
        let sidebar_for_select = sidebar.clone();
        sidebar.connect_row_selected(move |_, row| {
            let Some(row) = row else { return };
            let id = row.widget_name();
            if let Some(index) = wizard.pages.iter().position(|page| page.spec.id == id) {
                select_page(&wizard, &sidebar_for_select, index);
            }
        });
    }
    {
        let wizard = wizard.clone();
        let sidebar = sidebar.clone();
        let back = back.clone();
        back.connect_clicked(move |_| {
            let current = wizard.current.get();
            if current > 0 {
                select_page(&wizard, &sidebar, current - 1);
            }
        });
    }
    {
        let wizard = wizard.clone();
        let sidebar = sidebar.clone();
        let next_button = next.clone();
        next.connect_clicked(move |_| {
            if wizard.config.borrow().setup_completed {
                std::process::Command::new("nix-settings-gui").spawn().ok();
                return;
            }
            let current = wizard.current.get();
            if current + 1 < wizard.pages.len() {
                select_page(&wizard, &sidebar, current + 1);
            } else {
                finish(&wizard, &sidebar, &next_button);
            }
        });
    }
    {
        let wizard = wizard.clone();
        let sidebar = sidebar.clone();
        let next_button = next.clone();
        reset_row.connect_activated(move |_| {
            let mut config = wizard.config.borrow_mut();
            config.reset();
            let _ = config.save();
            drop(config);
            show_wizard(&wizard, &sidebar, &next_button);
        });
    }
    {
        let wizard = wizard.clone();
        rebuild_row.connect_activated(move |_| {
            run_rebuild(&wizard);
        });
    }

    window.present();
}

fn select_page(wizard: &Rc<Wizard>, sidebar: &gtk::ListBox, index: usize) {
    if wizard.pages.is_empty() {
        return;
    }
    let index = index.min(wizard.pages.len() - 1);
    wizard.current.set(index);
    let page = &wizard.pages[index];
    wizard.page_stack.set_visible_child_name(page.spec.id);
    if let Some(row) = sidebar.row_at_index(index as i32) {
        sidebar.select_row(Some(&row));
    }
    wizard
        .status
        .set_text(&format!("Step {} of {}", index + 1, wizard.pages.len()));
}

fn finish(wizard: &Rc<Wizard>, sidebar: &gtk::ListBox, next: &gtk::Button) {
    let mut edits = Vec::new();
    for page in &wizard.pages {
        for (record, editor) in &page.controls {
            edits.extend(editor.edits(record));
        }
    }
    match edit::apply(&edits, &wizard.flake) {
        Ok(files) => {
            let mut config = wizard.config.borrow_mut();
            config.mark_completed();
            let _ = config.save();
            drop(config);
            wizard.status.set_text(&format!("Saved {files} file(s)"));
            show_next_steps(wizard, sidebar, next);
        }
        Err(error) => {
            wizard.status.set_text(&format!("Cannot save: {error}"));
        }
    }
}

fn run_rebuild(wizard: &Rc<Wizard>) {
    let gui = NixSettingsGuiConfig::load().unwrap_or_default();
    let config = wizard.config.borrow();
    let command = gui
        .rebuild_command
        .clone()
        .or_else(|| config.rebuild_command.clone())
        .unwrap_or_else(|| {
            format!(
                "sudo nixos-rebuild switch --flake {}#$(hostname)",
                wizard.flake
            )
        });
    let terminal = gui
        .terminal_command
        .clone()
        .or_else(|| config.terminal_command.clone())
        .unwrap_or_else(|| "ghostty -e zsh -c".to_string());
    drop(config);
    let full = if terminal.contains("{cmd}") {
        terminal.replace("{cmd}", &command)
    } else {
        format!(
            "{} {}",
            terminal,
            nix_settings_gui::apply::shell_quote(&command)
        )
    };
    wizard.status.set_text("Rebuilding...");
    if let Err(error) = std::process::Command::new("sh")
        .arg("-c")
        .arg(full)
        .spawn()
    {
        wizard
            .status
            .set_text(&format!("Cannot rebuild: {error}"));
    }
}

fn show_wizard(wizard: &Rc<Wizard>, sidebar: &gtk::ListBox, next: &gtk::Button) {
    if wizard.pages.is_empty() {
        return;
    }
    next.set_label("Next");
    select_page(wizard, sidebar, 0);
}

fn show_next_steps(wizard: &Rc<Wizard>, _sidebar: &gtk::ListBox, next: &gtk::Button) {
    wizard.page_stack.set_visible_child_name("next-steps");
    wizard.status.set_text("Setup is complete");
    next.set_label("Open settings");
}

fn current_style_index(editors: &[(String, Editor)]) -> usize {
    let mut bar = None;
    let mut dock = false;
    for (path, editor) in editors {
        if path == "ojuny.dux.barPreset" {
            if let Editor::Dropdown { combo, choices } = editor {
                if let Some(choice) = choices.get(combo.selected() as usize) {
                    bar = Some(choice.trim_matches('"').to_string());
                }
            }
        } else if path == "desktop.ojuny.dux.dockConfigs.dock.enabled" {
            if let Editor::Switch(switch) = editor {
                dock = switch.is_active();
            }
        }
    }
    match (bar.as_deref(), dock) {
        (Some("taskbar"), _) => 0,
        (_, true) => 1,
        (Some("simple-island"), _) => 3,
        _ => 2,
    }
}

fn current_nixpkgs_channel(flake: &str) -> Option<String> {
    let document = flake_inputs::load(&format!("{flake}/flake.nix")).ok()?;
    let input = document.inputs.iter().find(|input| input.name == "nixpkgs")?;
    let url = input.url.as_deref()?;
    let channel = url.rsplit('/').next()?.trim();
    (!channel.is_empty()).then(|| channel.to_string())
}

fn set_nixpkgs_channel(flake: &str, channel: &str) -> Result<(), String> {
    let path = format!("{flake}/flake.nix");
    let document = flake_inputs::load(&path)?;
    let mut input = flake_inputs::FlakeInput::new("nixpkgs");
    input.url = Some(format!("github:NixOS/nixpkgs/{channel}"));
    let edits = document.edits_for_upsert(&input)?;
    edit::apply(&edits, flake)?;
    Ok(())
}

fn gpu_override_needed(flake: &str) -> bool {
    let path = std::path::Path::new(flake).join("modules/07-system/facter.json");
    let Ok(text) = std::fs::read_to_string(path) else {
        return true;
    };
    let Some(detection) = facter_detect::detect_report(&text) else {
        return true;
    };
    detection.state != "matched"
}

fn build_next_steps() -> (gtk::Box, adw::ActionRow, adw::ActionRow) {
    let root = gtk::Box::new(gtk::Orientation::Vertical, 12);
    root.set_margin_top(24);
    root.set_margin_bottom(24);
    root.set_margin_start(32);
    root.set_margin_end(32);
    root.append(
        &gtk::Label::builder()
            .label("Next steps")
            .xalign(0.0)
            .css_classes(["title-1"])
            .build(),
    );
    root.append(
        &gtk::Label::builder()
            .label("Your first-boot setup is finished. Open a resource below, or run the wizard again.")
            .xalign(0.0)
            .wrap(true)
            .css_classes(["dim-label"])
            .build(),
    );
    let group = adw::PreferencesGroup::new();
    let rebuild = adw::ActionRow::builder()
        .title("Rebuild the system")
        .subtitle("Apply the saved settings and rebuild now")
        .activatable(true)
        .build();
    group.add(&rebuild);
    let actions = [
        ("Update your system", "Open the settings GUI", "nix-settings-gui"),
        ("Read the manual", "Open the Ojuny manual", "ojuny-manual"),
        (
            "Visit the website",
            "Open the what-is-ojuny page",
            "xdg-open https://ojuny-project.github.io/",
        ),
    ];
    for (title, subtitle, command) in actions {
        let row = adw::ActionRow::builder()
            .title(title)
            .subtitle(subtitle)
            .activatable(true)
            .build();
        row.connect_activated(move |_| {
            let mut parts = command.split_whitespace();
            if let Some(program) = parts.next() {
                let _ = std::process::Command::new(program).args(parts).spawn();
            }
        });
        group.add(&row);
    }
    let reset = adw::ActionRow::builder()
        .title("Re-run the setup wizard")
        .subtitle("Reset completion and start from the first page")
        .activatable(true)
        .build();
    group.add(&reset);
    root.append(&group);
    (root, reset, rebuild)
}
