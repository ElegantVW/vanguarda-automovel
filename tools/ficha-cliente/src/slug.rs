pub fn slug(name: &str) -> String {
    let mut out = String::new();
    let mut last_underscore = false;
    for c in name.chars() {
        let mapped = fold(c);
        match mapped {
            Some('_') => {
                if !out.is_empty() && !last_underscore {
                    out.push('_');
                    last_underscore = true;
                }
            }
            Some(ch) => {
                out.push(ch);
                last_underscore = false;
            }
            None => {}
        }
    }
    while out.ends_with('_') {
        out.pop();
    }
    if out.is_empty() {
        "SemNome".to_string()
    } else {
        out
    }
}

fn fold(c: char) -> Option<char> {
    let lower = c.to_lowercase().next()?;
    let mapped = match lower {
        'á' | 'à' | 'ã' | 'â' => 'a',
        'é' | 'è' | 'ê' => 'e',
        'í' | 'ì' | 'î' => 'i',
        'ó' | 'ò' | 'õ' | 'ô' => 'o',
        'ú' | 'ù' | 'û' | 'ü' => 'u',
        'ç' => 'c',
        'ñ' => 'n',
        c if c.is_ascii_alphanumeric() => c,
        ' ' | '-' | '.' | '/' | '\\' => '_',
        _ => return None,
    };
    if mapped == '_' {
        Some('_')
    } else if c.is_uppercase() {
        mapped.to_uppercase().next()
    } else {
        Some(mapped)
    }
}

#[cfg(test)]
mod tests {
    use super::slug;

    #[test]
    fn ines() {
        assert_eq!(slug("Inês"), "Ines");
    }

    #[test]
    fn alvaro() {
        assert_eq!(slug("Álvaro Salvador"), "Alvaro_Salvador");
    }
}
