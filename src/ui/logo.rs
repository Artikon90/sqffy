use super::{paint_cyan, paint_dim};
use crate::i18n::t;

/// Large block logo — readable at typical terminal sizes (~72+ cols).
const LOGO: &[&str] = &[
    r"███████╗ ██████╗ ███████╗███████╗██╗   ██╗",
    r"██╔════╝██╔═══██╗██╔════╝██╔════╝╚██╗ ██╔╝",
    r"███████╗██║   ██║█████╗  █████╗   ╚████╔╝ ",
    r"╚════██║██║▄▄ ██║██╔══╝  ██╔══╝    ╚██╔╝  ",
    r"███████║╚██████╔╝██║     ██║        ██║   ",
    r"╚══════╝ ╚══▀▀═╝ ╚═╝     ╚═╝        ╚═╝   ",
];

pub fn print_banner() {
    println!();
    for line in LOGO {
        println!("{}", paint_cyan(line));
    }
    println!(
        "{}",
        paint_dim(&format!("  {}  ·  {}", t("app.name"), t("app.tagline")))
    );
    println!();
}

/// One-line brand for REPL chrome after connect.
pub fn print_compact_brand(connection: &str, driver: &str, hint: &str) {
    println!(
        "{} {}  {}",
        paint_cyan("◆"),
        paint_cyan(&t("app.name")),
        paint_dim(&format!("{connection} · {driver} · {hint}"))
    );
}
