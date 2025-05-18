//! Interactive connection picker shown when `sqf` is launched without a target.

use crate::connections::{self, ConnectionStore, SavedConnection};
use crate::db::{CreateKind, ConnectionSpec, DriverRegistry, LocalStatus};
use crate::error::{Error, Result};
use crate::i18n::{t, tr};
use crate::provision::{self, docker_available};
use crate::ui::{self, FilterOption, MenuItem, Panel, SelectOutcome};
use std::io::{self, Write};
use std::sync::Arc;

pub enum PickerOutcome {
    Connect(SavedConnection),
    Quit,
}

enum Entry {
    Connect(SavedConnection),
    Add,
    Create,
    Quit,
}

/// Run the startup menu. Saves new connections to the store.
pub fn run(registry: &DriverRegistry, store: &mut ConnectionStore) -> Result<PickerOutcome> {
    loop {
        ui::print_banner();
        let (panel, entries) = build_menu(store);

        match ui::interactive_select(&panel, 0)? {
            SelectOutcome::Quit => return Ok(PickerOutcome::Quit),
            SelectOutcome::Confirmed(idx) => match entries.get(idx) {
                Some(Entry::Quit) | None => return Ok(PickerOutcome::Quit),
                Some(Entry::Connect(conn)) => {
                    let mut conn = conn.clone();
                    conn.touch();
                    store.upsert(conn.clone());
                    store.save()?;
                    return Ok(PickerOutcome::Connect(conn));
                }
                Some(Entry::Add) => {
                    if let Some(conn) = wizard_add(registry, store)? {
                        return Ok(PickerOutcome::Connect(conn));
                    }
                }
                Some(Entry::Create) => {
                    if let Some(conn) = wizard_create(registry, store)? {
                        return Ok(PickerOutcome::Connect(conn));
                    }
                }
            },
        }
    }
}

fn build_menu(store: &ConnectionStore) -> (Panel, Vec<Entry>) {
    let footer = menu_footer();

    if store.connections.is_empty() {
        let items = vec![
            MenuItem::action("1", t("picker.create")),
            MenuItem::action("2", t("picker.add")),
            MenuItem::action("q", t("picker.quit")),
        ];
        let entries = vec![Entry::Create, Entry::Add, Entry::Quit];
        let panel = Panel::new(t("picker.empty_title"))
            .subtitle(t("picker.empty_hint"))
            .items(items)
            .footer(footer);
        return (panel, entries);
    }

    let listed = store.sorted();
    let mut items = Vec::with_capacity(listed.len() + 3);
    let mut entries = Vec::with_capacity(listed.len() + 3);

    for (idx, conn) in listed.iter().enumerate() {
        let hint = format!("{} · {}", conn.driver, conn.target);
        items.push(MenuItem::connection(
            (idx + 1).to_string(),
            conn.name.clone(),
            hint,
        ));
        entries.push(Entry::Connect((*conn).clone()));
    }

    let add_n = listed.len() + 1;
    let create_n = listed.len() + 2;
    items.push(MenuItem::action(add_n.to_string(), t("picker.add")));
    entries.push(Entry::Add);
    items.push(MenuItem::action(create_n.to_string(), t("picker.create")));
    entries.push(Entry::Create);
    items.push(MenuItem::action("q", t("picker.quit")));
    entries.push(Entry::Quit);

    let panel = Panel::new(t("picker.title"))
        .subtitle(t("picker.list_hint"))
        .items(items)
        .footer(footer);
    (panel, entries)
}

fn menu_footer() -> String {
    let hints = t("picker.hint_keys");
    match connections::store_path() {
        Ok(path) => format!(
            "{}  ·  {}",
            hints,
            tr(
                "picker.store_path",
                &[("path", &path.display().to_string())]
            )
        ),
        Err(_) => hints,
    }
}

fn wizard_create(
    registry: &DriverRegistry,
    store: &mut ConnectionStore,
) -> Result<Option<SavedConnection>> {
    println!();
    ui::rule();
    println!("{}", ui::paint_bold(&t("picker.creating")));
    ui::rule();

    let driver_id = match select_db_type(registry)? {
        Some(id) => id,
        None => {
            println!("{}", ui::paint_dim(&t("picker.cancelled")));
            return Ok(None);
        }
    };
    let driver = registry.get(&driver_id).cloned().ok_or_else(|| {
        let known: Vec<_> = registry.list().into_iter().map(|(id, _)| id).collect();
        Error::msg(tr(
            "drivers.unknown",
            &[("driver", &driver_id), ("known", &known.join(", "))],
        ))
    })?;

    let info = driver.provision();
    println!("{}", ui::paint_dim(&t(info.help_key)));

    let name = match prompt(&t("picker.ask_name"))? {
        Some(v) if !v.is_empty() => v,
        _ => {
            println!("{}", ui::paint_dim(&t("picker.cancelled")));
            return Ok(None);
        }
    };

    match info.create_kind {
        CreateKind::File => create_sqlite_file(registry, store, name),
        CreateKind::Server => create_server_db(registry, store, driver, name),
    }
}

fn create_sqlite_file(
    registry: &DriverRegistry,
    store: &mut ConnectionStore,
    name: String,
) -> Result<Option<SavedConnection>> {
    let default_path = connections::data_dir()?
        .join(format!("{}.db", sanitize_filename(&name)))
        .display()
        .to_string();
    let path_input = match prompt(&tr(
        "picker.ask_path_default",
        &[("default", &default_path)],
    ))? {
        Some(v) if !v.is_empty() => v,
        _ => default_path,
    };

    let path = connections::resolve_sqlite_path(&path_input);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let spec = ConnectionSpec {
        driver: "sqlite".into(),
        target: path.display().to_string(),
        url: None,
    };
    let _db = registry.open(&spec)?;
    println!(
        "{}",
        ui::paint_green(&tr(
            "picker.created_file",
            &[("path", &path.display().to_string())]
        ))
    );

    let conn = connections::new_saved(name, "sqlite", path.display().to_string(), None);
    finish_save(store, conn)
}

fn create_server_db(
    registry: &DriverRegistry,
    store: &mut ConnectionStore,
    driver: Arc<dyn crate::db::Driver>,
    name: String,
) -> Result<Option<SavedConnection>> {
    let info = driver.provision();
    let status = driver.probe_local();

    if let LocalStatus::Ready { host, port } = &status {
        println!(
            "{}",
            ui::paint_green(&tr(
                "provision.local_found",
                &[("host", host), ("port", &port.to_string())],
            ))
        );
        let url = match prompt(&t("provision.ask_url_local"))? {
            Some(v) if !v.is_empty() => v,
            _ => {
                // Build a guess URL from docker template if present
                if let Some(docker) = &info.docker {
                    docker
                        .url_template
                        .replace("{host}", host)
                        .replace("{port}", &port.to_string())
                } else {
                    println!("{}", ui::paint_dim(&t("picker.cancelled")));
                    return Ok(None);
                }
            }
        };
        let spec = ConnectionSpec::parse(Some(&url), Some(driver.id()))?;
        let _db = registry.open(&spec)?;
        let conn = connections::new_saved(name, spec.driver.clone(), spec.target.clone(), spec.url);
        return finish_save(store, conn);
    }

    println!("{}", ui::paint_yellow(&t("provision.local_missing")));
    let choice = match select_provision_action(&info)? {
        Some(c) => c,
        None => {
            println!("{}", ui::paint_dim(&t("picker.cancelled")));
            return Ok(None);
        }
    };

    match choice {
        ProvisionAction::Docker => {
            if !docker_available() {
                println!("{}", ui::paint_red(&t("provision.docker_missing")));
                return Ok(None);
            }
            let docker = info
                .docker
                .as_ref()
                .ok_or_else(|| Error::msg(t("provision.docker_unsupported")))?;
            let confirm = prompt(&t("provision.confirm_docker"))?;
            if !matches_yes(confirm.as_deref()) {
                println!("{}", ui::paint_dim(&t("picker.cancelled")));
                return Ok(None);
            }
            let run = provision::pull_and_run(docker, &name)?;
            println!(
                "{}",
                ui::paint_green(&tr(
                    "provision.docker_ok",
                    &[("name", &run.container_name), ("url", &run.url)],
                ))
            );
            let spec = ConnectionSpec::parse(Some(&run.url), Some(driver.id()))?;
            let _db = registry.open(&spec)?;
            let conn =
                connections::new_saved(name, spec.driver.clone(), spec.target.clone(), spec.url);
            finish_save(store, conn)
        }
        ProvisionAction::Download => {
            let confirm = prompt(&t("provision.confirm_download"))?;
            if !matches_yes(confirm.as_deref()) {
                println!("{}", ui::paint_dim(&t("picker.cancelled")));
                return Ok(None);
            }
            if let Some(pkg) = info.winget_id {
                match provision::try_winget_install(pkg)? {
                    true => {
                        println!("{}", ui::paint_dim(&t("provision.download_restart_hint")));
                    }
                    false => {
                        if let Some(url) = info.download_url {
                            println!(
                                "{}",
                                ui::paint_yellow(&tr("provision.download_manual", &[("url", url)]))
                            );
                        }
                    }
                }
            } else if let Some(url) = info.download_url {
                println!(
                    "{}",
                    ui::paint_yellow(&tr("provision.download_manual", &[("url", url)]))
                );
            }
            // After download attempt, ask for connection URL
            let url = match prompt(&t("provision.ask_url_after"))? {
                Some(v) if !v.is_empty() => v,
                _ => {
                    println!("{}", ui::paint_dim(&t("picker.cancelled")));
                    return Ok(None);
                }
            };
            let spec = ConnectionSpec::parse(Some(&url), Some(driver.id()))?;
            let _db = registry.open(&spec)?;
            let conn =
                connections::new_saved(name, spec.driver.clone(), spec.target.clone(), spec.url);
            finish_save(store, conn)
        }
        ProvisionAction::Manual => {
            let url = match prompt(&t("provision.ask_url_manual"))? {
                Some(v) if !v.is_empty() => v,
                _ => {
                    println!("{}", ui::paint_dim(&t("picker.cancelled")));
                    return Ok(None);
                }
            };
            let spec = ConnectionSpec::parse(Some(&url), Some(driver.id()))?;
            let _db = registry.open(&spec)?;
            let conn =
                connections::new_saved(name, spec.driver.clone(), spec.target.clone(), spec.url);
            finish_save(store, conn)
        }
    }
}

#[derive(Clone, Copy)]
enum ProvisionAction {
    Docker,
    Download,
    Manual,
}

fn select_provision_action(info: &crate::db::ProvisionInfo) -> Result<Option<ProvisionAction>> {
    let mut options = Vec::new();
    let mut actions = Vec::new();
    if info.docker.is_some() {
        options.push(
            FilterOption::new("docker", t("provision.action.docker"))
                .with_hint(t("provision.action.docker_hint")),
        );
        actions.push(ProvisionAction::Docker);
    }
    options.push(
        FilterOption::new("download", t("provision.action.download"))
            .with_hint(t("provision.action.download_hint")),
    );
    actions.push(ProvisionAction::Download);
    options.push(
        FilterOption::new("manual", t("provision.action.manual"))
            .with_hint(t("provision.action.manual_hint")),
    );
    actions.push(ProvisionAction::Manual);

    match ui::interactive_filter_select(
        &t("provision.action_title"),
        &t("provision.action_hint"),
        &options,
    )? {
        SelectOutcome::Confirmed(idx) => Ok(actions.get(idx).copied()),
        SelectOutcome::Quit => Ok(None),
    }
}

fn matches_yes(answer: Option<&str>) -> bool {
    matches!(
        answer.map(|s| s.trim().to_ascii_lowercase()).as_deref(),
        Some("y") | Some("yes") | Some("д") | Some("да") | Some("")
    )
}

fn wizard_add(
    registry: &DriverRegistry,
    store: &mut ConnectionStore,
) -> Result<Option<SavedConnection>> {
    println!();
    ui::rule();
    println!("{}", ui::paint_bold(&t("picker.adding")));
    ui::rule();

    let name = match prompt(&t("picker.ask_name"))? {
        Some(v) if !v.is_empty() => v,
        _ => {
            println!("{}", ui::paint_dim(&t("picker.cancelled")));
            return Ok(None);
        }
    };

    let driver = match select_db_type(registry)? {
        Some(id) => id,
        None => {
            println!("{}", ui::paint_dim(&t("picker.cancelled")));
            return Ok(None);
        }
    };

    let target = match prompt(&t("picker.ask_target"))? {
        Some(v) if !v.is_empty() => v,
        _ => {
            println!("{}", ui::paint_dim(&t("picker.cancelled")));
            return Ok(None);
        }
    };

    let spec = if driver == "sqlite" && !target.contains("://") && target != ":memory:" {
        ConnectionSpec {
            driver: driver.clone(),
            target: connections::resolve_sqlite_path(&target)
                .display()
                .to_string(),
            url: None,
        }
    } else {
        ConnectionSpec::parse(Some(&target), Some(&driver))?
    };

    let _db = registry.open(&spec)?;
    let conn = connections::new_saved(name, spec.driver.clone(), spec.target.clone(), spec.url);
    finish_save(store, conn)
}

fn finish_save(
    store: &mut ConnectionStore,
    conn: SavedConnection,
) -> Result<Option<SavedConnection>> {
    println!(
        "{}",
        ui::paint_green(&tr("picker.saved", &[("name", &conn.name)]))
    );
    store.upsert(conn.clone());
    store.save()?;
    Ok(Some(conn))
}

fn select_db_type(registry: &DriverRegistry) -> Result<Option<String>> {
    let options: Vec<FilterOption> = registry
        .list()
        .into_iter()
        .map(|(id, desc)| {
            FilterOption::new(id, db_type_label(id)).with_hint(desc)
        })
        .collect();
    if options.is_empty() {
        return Ok(Some("sqlite".into()));
    }
    println!();
    match ui::interactive_filter_select(
        &t("picker.db_type_title"),
        &t("picker.db_type_hint"),
        &options,
    )? {
        SelectOutcome::Confirmed(idx) => Ok(options.get(idx).map(|o| o.id.clone())),
        SelectOutcome::Quit => Ok(None),
    }
}

fn db_type_label(id: &str) -> String {
    match id {
        "sqlite" => t("picker.db_type.sqlite"),
        "postgres" => t("picker.db_type.postgres"),
        "mysql" => t("picker.db_type.mysql"),
        other => other.to_string(),
    }
}

fn prompt(label: &str) -> Result<Option<String>> {
    ui::prompt_line(label).map_err(Into::into)
}

fn sanitize_filename(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if s.is_empty() {
        "database".into()
    } else {
        s
    }
}

pub fn connect_message(conn: &SavedConnection) {
    println!();
    println!(
        "{}",
        ui::paint_dim(&tr(
            "picker.connecting",
            &[("name", &conn.name), ("driver", &conn.driver)]
        ))
    );
    let _ = io::stdout().flush();
}
