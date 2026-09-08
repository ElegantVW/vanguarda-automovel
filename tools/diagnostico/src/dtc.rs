/// Subconjunto J2012 público (PT curto). Códigos de fabricante: `None`.
pub fn describe_dtc(code: &str) -> Option<&'static str> {
    match code.to_ascii_uppercase().as_str() {
        "P0300" => Some("falha de ignição múltipla"),
        "P0301" => Some("falha de ignição cilindro 1"),
        "P0420" => Some("eficiência do catalisador abaixo do limiar"),
        "P2463" => Some("filtro de partículas — acumulação de fuligem"),
        "P2598" => Some("turbo — posição do actuador"),
        "P0833" => Some("interruptor da embraiagem"),
        "P068A" => Some("relé principal ECM — desenergização"),
        "P2563" => Some("posição do actuador de sobrealimentação"),
        "U0401" => Some("dados inválidos recebidos do ECM"),
        "U0253" => Some("comunicação perdida com módulo"),
        "U0184" => Some("comunicação perdida com rádio"),
        "B1182" => Some("avaria no módulo de carroçaria"),
        "B124D" => Some("avaria no módulo de carroçaria"),
        _ => None,
    }
}
