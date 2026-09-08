/// Normalise form fields. Never invent values.

pub fn is_placeholder(s: &str) -> bool {
    let t = s.trim();
    t.is_empty()
        || t.eq_ignore_ascii_case("n/a")
        || t.eq_ignore_ascii_case("na")
        || t == "[●]"
        || t == "—"
        || t.eq_ignore_ascii_case("aasas")
}

pub fn blank_placeholder(s: &str) -> String {
    if is_placeholder(s) {
        String::new()
    } else {
        s.trim().to_string()
    }
}

pub fn looks_like_dtc_dump(s: &str) -> bool {
    let l = s.to_lowercase();
    if l.contains("release 2021") || l.contains("code4bin") || l.contains("autocom") {
        return true;
    }
    let has_code = l.contains("p161")
        || l.contains("p16")
        || l.contains("01415")
        || l.contains("circuito")
        || l.contains("vela de incandesc");
    has_code && (l.contains("presente") || l.contains("permanente") || l.contains("motor "))
}

pub fn format_plate(s: &str) -> String {
    let t: String = s
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_uppercase())
        .collect();
    if t.len() == 6 {
        format!("{}-{}-{}", &t[0..2], &t[2..4], &t[4..6])
    } else if is_placeholder(s) {
        String::new()
    } else {
        s.trim().to_string()
    }
}

pub fn format_vin(s: &str) -> String {
    if is_placeholder(s) {
        return String::new();
    }
    let t: String = s
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| {
            let u = c.to_ascii_uppercase();
            if u == 'I' {
                '1'
            } else {
                u
            }
        })
        .collect();
    if t.contains("RELEASE") || t.contains("KILOMET") {
        return String::new();
    }
    if t.len() == 17 {
        t
    } else {
        s.trim().to_string()
    }
}

pub fn format_km(s: &str) -> String {
    if is_placeholder(s) {
        return String::new();
    }
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        s.trim().to_string()
    } else {
        digits
    }
}

pub fn format_nif(s: &str) -> String {
    if is_placeholder(s) {
        return String::new();
    }
    let d: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    if d.len() == 9 {
        d
    } else {
        s.trim().to_string()
    }
}

/// Check digit PT: pesos 9..2, resto 11. Vazio é válido (ainda não preenchido).
pub fn nif_valido(s: &str) -> bool {
    let d: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    if d.is_empty() {
        return true;
    }
    if d.len() != 9 {
        return false;
    }
    let digits: Vec<u32> = d.chars().filter_map(|c| c.to_digit(10)).collect();
    let sum: u32 = (0..8).map(|i| digits[i] * (9 - i as u32)).sum();
    let rem = sum % 11;
    let check = if rem < 2 { 0 } else { 11 - rem };
    digits[8] == check
}

pub fn format_phone(s: &str) -> String {
    if is_placeholder(s) {
        return String::new();
    }
    let mut out = String::new();
    for c in s.chars() {
        if c.is_ascii_digit() || c == '+' {
            out.push(c);
        } else if c == ' ' || c == '-' {
            if !out.ends_with(' ') && !out.is_empty() {
                out.push(' ');
            }
        }
    }
    out.trim().to_string()
}

pub fn format_date(s: &str) -> String {
    if is_placeholder(s) {
        return String::new();
    }
    let d: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    if d.len() == 8 {
        format!("{}/{}/{}", &d[0..2], &d[2..4], &d[4..8])
    } else {
        s.trim().to_string()
    }
}

pub fn format_named(label: &str, value: &str) -> String {
    match label {
        "Matrícula" | "Veículo / matrícula" => format_plate(value),
        "VIN" => format_vin(value),
        "Km" | "Quilometragem" => format_km(value),
        "NIF" => format_nif(value),
        "Telemóvel" => format_phone(value),
        "Data" | "Data nascimento" | "Data de início" | "Última visita" => format_date(value),
        _ => blank_placeholder(value),
    }
}

pub fn field_hint(label: &str) -> Option<&'static str> {
    match label {
        "Matrícula" | "Veículo / matrícula" => Some("AA-00-AA"),
        "VIN" => Some("17 caracteres"),
        "NIF" => Some("9 dígitos"),
        "Km" | "Quilometragem" => Some("só números"),
        "Telemóvel" => Some("+351 …"),
        "Data" | "Data nascimento" | "Data de início" => Some("DD/MM/AAAA"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plate_vin_na_dump() {
        assert_eq!(format_plate("60hu86"), "60-HU-86");
        assert_eq!(format_vin("WVGZZZITZ9W034244").len(), 17);
        assert!(format_vin("WVGZZZITZ9W034244").starts_with("WVGZZZ1"));
        assert!(is_placeholder("N/A"));
        assert!(nif_valido(""));
        assert!(nif_valido("123456789"));
        assert!(!nif_valido("123456788"));
        assert!(looks_like_dtc_dump(
            "Volkswagen - Touran Motor P161A Presente circuito aberto"
        ));
        assert!(!looks_like_dtc_dump("Touran"));
    }
}
