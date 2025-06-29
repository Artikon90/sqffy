use fake::Fake;
use fake::faker::internet::en::SafeEmail;
use fake::faker::lorem::en::Word;
use fake::faker::name::en::Name;
use rand::Rng;

/// Produce a SQL literal for a column type hint.
pub fn gen_value(type_name: &str, rng: &mut impl Rng) -> String {
    let t = type_name.to_ascii_uppercase();
    if t.contains("BOOL") {
        return if rng.gen_bool(0.5) {
            "TRUE".into()
        } else {
            "FALSE".into()
        };
    }
    if t.contains("INT") || t.contains("SERIAL") || t == "BIGINT" || t == "SMALLINT" {
        return rng.gen_range(1..10_000).to_string();
    }
    if t.contains("REAL")
        || t.contains("DOUBLE")
        || t.contains("FLOAT")
        || t.contains("NUMERIC")
        || t.contains("DECIMAL")
        || t.contains("MONEY")
    {
        let v: f64 = rng.gen_range(0.0..10_000.0);
        return format!("{v:.2}");
    }
    if t.contains("DATE") && !t.contains("TIME") {
        let y = rng.gen_range(2000..2026);
        let m = rng.gen_range(1..=12);
        let d = rng.gen_range(1..=28);
        return format!("'{y:04}-{m:02}-{d:02}'");
    }
    if t.contains("TIME") || t.contains("TIMESTAMP") {
        return "'2024-01-15 12:00:00'".into();
    }
    if t.contains("UUID") {
        return format!(
            "'{:08x}-{:04x}-{:04x}-{:04x}-{:012x}'",
            rng.r#gen::<u32>(),
            rng.r#gen::<u16>(),
            rng.r#gen::<u16>(),
            rng.r#gen::<u16>(),
            rng.r#gen::<u64>() & 0xffff_ffff_ffff
        );
    }
    if t.contains("EMAIL") || t.contains("VARCHAR") && rng.gen_bool(0.2) {
        let email: String = SafeEmail().fake_with_rng(rng);
        return format!("'{}'", email.replace('\'', "''"));
    }
    if t.contains("NAME") || rng.gen_bool(0.3) {
        let name: String = Name().fake_with_rng(rng);
        return format!("'{}'", name.replace('\'', "''"));
    }
    let word: String = Word().fake_with_rng(rng);
    format!("'{}'", word.replace('\'', "''"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    #[test]
    fn int_is_numeric_literal() {
        let mut rng = StdRng::seed_from_u64(1);
        let v = gen_value("INTEGER", &mut rng);
        assert!(v.parse::<i64>().is_ok(), "{v}");
    }

    #[test]
    fn text_is_quoted() {
        let mut rng = StdRng::seed_from_u64(2);
        let v = gen_value("TEXT", &mut rng);
        assert!(v.starts_with('\''), "{v}");
    }
}
