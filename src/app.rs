use crate::cli::{Cli, Command};
use crate::complete::{Completer, SchemaCache};
use crate::config::Config;
use crate::connections::{self, ConnectionStore, SavedConnection};
use crate::db::{self, ConnectionSpec, Database, DriverRegistry};
use crate::error::{Error, Result};
use crate::i18n::{self, t, tr};
use crate::mcp;
use crate::meta::{self, MetaCommand};
use crate::picker::{self, PickerOutcome};
use crate::pool::ConnectionPool;
use crate::render;
use crate::repl::{ReadOutcome, Repl};
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::time::Instant;

pub struct App {
    db: Box<dyn Database>,
    config: Config,
    repl: Repl,
    connection_name: String,
}

impl App {
    fn from_parts(
        db: Box<dyn Database>,
        config: Config,
        connection_name: String,
        script: Option<&str>,
    ) -> Result<Self> {
        let cache = SchemaCache::refresh(db.as_ref())?;
        let completer = Completer::new(cache);
        let repl = Repl::new(completer)?;
        let mut app = Self {
            db,
            config,
            repl,
            connection_name,
        };
        if let Some(script) = script {
            app.run_script(script)?;
        }
        Ok(app)
    }

    pub fn run_repl(&mut self) -> Result<()> {
        // Leave picker raw-mode fully; otherwise readline redraw glitches.
        let _ = crossterm::terminal::disable_raw_mode();

        println!();
        crate::ui::print_compact_brand(
            &self.connection_name,
            self.db.driver_id(),
            &tr(
                "app.prompt_help",
                &[("limit", &self.config.row_limit.to_string())],
            ),
        );
        crate::ui::rule();
        // Show object list immediately after connect (same as `.tables`).
        match self.db.list_tables() {
            Ok(tables) => {
                let rendered = render::tables_list(&tables);
                let _ = render::print_rendered(&rendered, &self.config);
            }
            Err(err) => {
                eprint!("{}", render::format_error(&err));
            }
        }
        println!();

        loop {
            // Plain prompt only — coloring is done in ReplHelper::highlight_prompt.
            // ANSI inside readline() breaks cursor geometry on Backspace/Delete.
            let prompt = format!("{}> ", prompt_name(self.db.display_label()));
            match self.repl.read_line(&prompt)? {
                ReadOutcome::Eof => break,
                ReadOutcome::Interrupt => {
                    println!("{}", crate::ui::paint_dim(&t("app.interrupt")));
                    continue;
                }
                ReadOutcome::Line(line) => {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }
                    if let Err(err) = self.dispatch(line) {
                        eprint!("{}", render::format_error(&err));
                    }
                    if line.starts_with('.') {
                        if let Ok(MetaCommand::Quit) = MetaCommand::parse(line) {
                            break;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn run_script(&mut self, script: &str) -> Result<()> {
        for chunk in split_script(script) {
            self.dispatch(&chunk)?;
        }
        Ok(())
    }

    fn dispatch(&mut self, line: &str) -> Result<()> {
        if line.starts_with('.') {
            let cmd = MetaCommand::parse(line)?;
            if matches!(cmd, MetaCommand::Quit) {
                return Ok(());
            }
            if let MetaCommand::Generate {
                table,
                count,
                yes,
                dry_run,
                fks,
            } = &cmd
            {
                return self.run_generate(table, *count, *yes, *dry_run, fks);
            }
            if let Some(rendered) = meta::execute(self.db.as_ref(), &cmd)? {
                render::print_rendered(&rendered, &self.config)?;
            }
            return Ok(());
        }

        let started = Instant::now();
        let result = self.db.execute_sql(line, self.config.row_limit)?;
        let rendered = render::from_query(&result, &self.config);
        render::print_rendered(&rendered, &self.config)?;
        println!(
            "{}",
            crate::ui::paint_dim(&tr(
                "app.time",
                &[("secs", &format!("{:.3}", started.elapsed().as_secs_f64()))]
            ))
        );

        let upper = line.to_ascii_uppercase();
        if upper.contains("CREATE")
            || upper.contains("DROP")
            || upper.contains("ALTER")
            || upper.contains("ATTACH")
        {
            let cache = SchemaCache::refresh(self.db.as_ref())?;
            self.repl.refresh_completer(Completer::new(cache));
        }

        Ok(())
    }

    fn run_generate(
        &mut self,
        table: &str,
        count: usize,
        yes: bool,
        dry_run: bool,
        fks: &[(String, String, String)],
    ) -> Result<()> {
        let schema = self.db.inspect_table(table)?;
        let mut plan = crate::generate::GeneratePlan::from_schema(&schema, true);
        for (col, rt, rc) in fks {
            plan.add_fk(col, rt, rc);
        }
        crate::generate::fill_fk_pools(self.db.as_mut(), &mut plan)?;
        let inserted = crate::generate::apply(self.db.as_mut(), &plan, count, dry_run, yes)?;
        if !dry_run {
            println!(
                "{}",
                crate::ui::paint_green(&tr(
                    "generate.done",
                    &[("count", &inserted.to_string()), ("table", table)],
                ))
            );
        }
        Ok(())
    }
}

fn prompt_name(label: &str) -> String {
    PathBuf::from(label)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(label)
        .to_string()
}

fn split_script(script: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut sql_buf = String::new();

    for line in script.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with('.') {
            flush_sql(&mut sql_buf, &mut parts);
            parts.push(trimmed.to_string());
            continue;
        }
        if !sql_buf.is_empty() {
            sql_buf.push(' ');
        }
        sql_buf.push_str(trimmed);
        // Flush complete statements ending with `;` (keep remainder in buffer).
        while let Some(idx) = sql_buf.find(';') {
            let stmt = sql_buf[..idx].trim().to_string();
            if !stmt.is_empty() {
                parts.push(stmt);
            }
            sql_buf = sql_buf[idx + 1..].trim_start().to_string();
        }
    }
    flush_sql(&mut sql_buf, &mut parts);

    if parts.is_empty() {
        let trimmed = script.trim();
        if !trimmed.is_empty() {
            parts.push(trimmed.to_string());
        }
    }
    parts
}

fn flush_sql(buf: &mut String, parts: &mut Vec<String>) {
    let stmt = buf.trim().to_string();
    buf.clear();
    if !stmt.is_empty() {
        parts.push(stmt);
    }
}

fn print_drivers(registry: &DriverRegistry) {
    println!("{}", t("drivers.available"));
    for (id, description) in registry.list() {
        println!("  {id:<12} {description}");
    }
}

fn open_saved(
    registry: &DriverRegistry,
    conn: &SavedConnection,
) -> Result<Box<dyn Database>> {
    picker::connect_message(conn);
    registry.open(&conn.to_spec())
}

fn maybe_save_direct(
    store: &mut ConnectionStore,
    spec: &ConnectionSpec,
    no_save: bool,
) -> Result<String> {
    let label = PathBuf::from(&spec.target)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(&spec.target)
        .to_string();

    if no_save {
        return Ok(label);
    }

    // Update last_used if same target already saved; otherwise add.
    if let Some(existing) = store
        .connections
        .iter_mut()
        .find(|c| c.driver == spec.driver && c.target == spec.target)
    {
        existing.touch();
        let name = existing.name.clone();
        store.save()?;
        return Ok(name);
    }

    let conn = connections::new_saved(
        label.clone(),
        spec.driver.clone(),
        spec.target.clone(),
        spec.url.clone(),
    );
    let name = conn.name.clone();
    store.upsert(conn);
    store.save()?;
    Ok(name)
}

/// Entry used by `main` after clap parse.
pub fn run(cli: Cli) -> Result<()> {
    i18n::init(cli.lang.as_deref());

    let registry = db::default_registry();
    if cli.list_drivers {
        print_drivers(&registry);
        return Ok(());
    }

    if let Some(Command::Mcp { port }) = &cli.subcommand {
        return run_mcp_standalone(&registry, *port);
    }

    let config = Config::default()
        .with_row_limit(cli.limit)
        .with_pager(!cli.no_pager);

    let mut store = ConnectionStore::load()?;
    let one_shot = cli.command.is_some();

    let (db, connection_name, active_spec) = if let Some(target) = cli.target.as_deref() {
        let spec = ConnectionSpec::parse(Some(target), cli.driver.as_deref())?;
        let db = registry.open(&spec)?;
        let name = maybe_save_direct(&mut store, &spec, cli.no_save || one_shot)?;
        (db, name, Some(spec))
    } else if one_shot {
        let spec = ConnectionSpec::parse(None, cli.driver.as_deref())?;
        let db = registry.open(&spec)?;
        (db, ":memory:".into(), Some(spec))
    } else {
        match picker::run(&registry, &mut store)? {
            PickerOutcome::Quit => {
                println!("{}", crate::ui::paint_dim(&t("picker.cancelled")));
                return Ok(());
            }
            PickerOutcome::Connect(conn) => {
                let spec = conn.to_spec();
                let db = open_saved(&registry, &conn)?;
                let _ = store.mark_used(&conn.id);
                (db, conn.name, Some(spec))
            }
        }
    };

    let mcp_stop = if let Some(port) = cli.mcp_port {
        let pool = ConnectionPool::new();
        let _ = pool.load_saved(&store, &registry);
        if let Some(spec) = &active_spec {
            if pool.get(Some(&connection_name)).is_err() {
                let _ = pool.open_spec(&connection_name, spec);
            }
            let _ = pool.set_default(&connection_name);
        }
        let stop = mcp::run_http_background(pool, port);
        println!(
            "{}",
            crate::ui::paint_dim(&format!(
                "MCP (read-only) on http://127.0.0.1:{port}/mcp · parallel with CLI"
            ))
        );
        Some(stop)
    } else {
        None
    };

    let mut app = App::from_parts(db, config, connection_name, cli.command.as_deref())?;
    let result = if one_shot {
        Ok(())
    } else {
        app.run_repl()
    };
    if let Some(stop) = mcp_stop {
        stop.store(true, Ordering::Relaxed);
    }
    result
}

fn run_mcp_standalone(registry: &DriverRegistry, port: Option<u16>) -> Result<()> {
    let store = ConnectionStore::load()?;
    let pool = ConnectionPool::new();
    let errors = pool.load_saved(&store, registry);
    for e in errors {
        eprintln!("mcp: skip connection: {e}");
    }
    if pool.len() == 0 {
        // Fallback in-memory so tools still answer.
        let spec = ConnectionSpec::parse(None, Some("sqlite"))?;
        pool.open_spec("memory", &spec)?;
    }
    eprintln!(
        "mcp: {} connection(s) ready",
        pool.len()
    );
    if let Some(port) = port {
        let stop = mcp::run_http_background(pool, port);
        eprintln!("mcp: HTTP mode — Ctrl+C to stop");
        loop {
            std::thread::sleep(std::time::Duration::from_secs(3600));
            if stop.load(Ordering::Relaxed) {
                break;
            }
        }
        Ok(())
    } else {
        mcp::run_stdio(pool)
    }
}

pub fn open_error(err: Error) -> ! {
    eprint!("{}", render::format_error(&err));
    std::process::exit(1);
}
