use std::fmt;

pub fn header(title: &str) {
    println!("\n{title}\n");
}

pub fn section(title: &str) {
    println!("{title}");
    println!("{}", "─".repeat(40));
}

pub fn label(label: &str, value: impl fmt::Display) {
    println!("  {label:<16} {value}");
}

pub fn item(icon: &str, message: &str) {
    println!("  {icon} {message}");
}

pub fn success(message: &str) {
    item("✓", message);
}

pub fn warn(message: &str) {
    item("⚠", message);
}

pub fn error(message: &str) {
    item("✗", message);
}

pub fn info(message: &str) {
    item("ℹ", message);
}

pub fn dim(message: &str) {
    println!("  {message}");
}

pub fn blank() {
    println!();
}

pub fn table_row(columns: &[(&str, bool)]) {
    for (i, (text, is_header)) in columns.iter().enumerate() {
        if i > 0 {
            print!("  ");
        }
        if *is_header {
            print!("{:<20}", text.to_uppercase());
        } else {
            print!("{text:<20}");
        }
    }
    println!();
}
