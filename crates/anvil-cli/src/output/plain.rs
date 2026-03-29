use std::fmt;

#[allow(dead_code)]
pub fn header(title: &str) {
    println!("\n{title}\n");
}

pub fn section(title: &str) {
    println!("{title}");
    println!("{}", "\u{2500}".repeat(40));
}

pub fn label(label: &str, value: impl fmt::Display) {
    println!("  {label:<16} {value}");
}

pub fn item(icon: &str, message: &str) {
    println!("  {icon} {message}");
}

pub fn success(message: &str) {
    item("\u{2713}", message);
}

pub fn warn(message: &str) {
    item("\u{26a0}", message);
}

pub fn error(message: &str) {
    item("\u{2717}", message);
}

#[allow(dead_code)]
pub fn info(message: &str) {
    item("\u{2139}", message);
}

#[allow(dead_code)]
pub fn dim(message: &str) {
    println!("  {message}");
}

pub fn blank() {
    println!();
}

#[allow(dead_code)]
pub fn table_row(columns: &[(&str, bool)]) {
    for (i, (text, _is_header)) in columns.iter().enumerate() {
        if i > 0 {
            print!("  ");
        }
        print!("{text:<20}");
    }
    println!();
}
