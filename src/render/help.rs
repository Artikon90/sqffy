use super::Rendered;
use crate::i18n::t;
use crate::ui::{footnote, section_heading, title_banner, CellStyle, DataTable, StyledCell};

struct HelpRow {
    command: &'static str,
    desc_key: &'static str,
}

struct HelpSection {
    title_key: &'static str,
    rows: &'static [HelpRow],
}

const SECTIONS: &[HelpSection] = &[
    HelpSection {
        title_key: "help.section.commands",
        rows: &[
            HelpRow {
                command: ".tables / .t",
                desc_key: "help.cmd.tables",
            },
            HelpRow {
                command: ".schema",
                desc_key: "help.cmd.schema",
            },
            HelpRow {
                command: ".schema <table> / .s",
                desc_key: "help.cmd.schema_table",
            },
            HelpRow {
                command: ".describe <table> / .d",
                desc_key: "help.cmd.describe",
            },
            HelpRow {
                command: ".generate <table> <n>",
                desc_key: "help.cmd.generate",
            },
            HelpRow {
                command: ".help / .h",
                desc_key: "help.cmd.help",
            },
            HelpRow {
                command: ".quit / .q",
                desc_key: "help.cmd.quit",
            },
        ],
    },
    HelpSection {
        title_key: "help.section.connect",
        rows: &[
            HelpRow {
                command: "sqf",
                desc_key: "help.cmd.sqf",
            },
            HelpRow {
                command: "sqf ./file.db",
                desc_key: "help.cmd.sqf_file",
            },
            HelpRow {
                command: "sqf --lang ru",
                desc_key: "help.cmd.sqf_lang",
            },
            HelpRow {
                command: "sqf --list-drivers",
                desc_key: "help.cmd.sqf_drivers",
            },
        ],
    },
];

/// Pretty multi-section help built on the shared [`DataTable`].
pub fn help_text() -> Rendered {
    let mut out = String::new();
    out.push_str(&title_banner(&t("help.title")));
    out.push('\n');
    out.push('\n');

    for (i, section) in SECTIONS.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str(&section_heading(&t(section.title_key)));
        out.push('\n');
        out.push_str(&section_table(section));
        out.push('\n');
    }

    out.push('\n');
    out.push_str(&footnote(&t("help.note.sql")));
    out.push('\n');
    out.push_str(&footnote(&t("help.note.pager")));
    out.push('\n');

    Rendered::plain(out)
}

fn section_table(section: &HelpSection) -> String {
    let mut table = DataTable::new().headers_styled([
        StyledCell::new(t("help.col.command"), CellStyle::Accent),
        StyledCell::new(t("help.col.description"), CellStyle::Bold),
    ]);

    for row in section.rows {
        table.push_row([
            StyledCell::accent(row.command),
            StyledCell::plain(t(row.desc_key)),
        ]);
    }

    table.render()
}
