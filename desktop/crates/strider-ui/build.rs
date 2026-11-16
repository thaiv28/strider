use std::{collections::BTreeMap, env, fs, path::PathBuf};

fn tokens(css: &str, marker: &str) -> BTreeMap<String, String> {
    let tail = css.split_once(marker).expect("Topo token block missing").1;
    let block = tail
        .split_once('{')
        .expect("Topo token block has no opening brace")
        .1
        .split_once('}')
        .expect("Topo token block has no closing brace")
        .0;
    block
        .lines()
        .filter_map(|line| {
            let (name, value) = line.trim().strip_prefix("--")?.split_once(':')?;
            Some((
                name.replace('-', "_"),
                value.trim().trim_end_matches(';').to_owned(),
            ))
        })
        .collect()
}
fn color(value: &str) -> String {
    if let Some(hex) = value.strip_prefix('#') {
        let integer = u32::from_str_radix(hex, 16).expect("invalid Topo hex color");
        format!("gpui::rgb({integer}).into()")
    } else {
        let parts: Vec<f32> = value
            .strip_prefix("rgba(")
            .and_then(|s| s.strip_suffix(')'))
            .expect("unsupported Topo color; expected hex or rgba")
            .split(',')
            .map(|s| s.trim().parse().expect("invalid Topo rgba component"))
            .collect();
        assert_eq!(parts.len(), 4, "Topo rgba needs four components");
        format!(
            "gpui::Rgba {{ r: {:?}, g: {:?}, b: {:?}, a: {:?} }}.into()",
            parts[0] / 255.,
            parts[1] / 255.,
            parts[2] / 255.,
            parts[3]
        )
    }
}
fn main() {
    let source = "../../../app/globals.css";
    println!("cargo:rerun-if-changed={source}");
    let css = fs::read_to_string(source).expect("cannot read Topo colors from app/globals.css");
    let topo = css
        .split_once("/* Topo tokens.")
        .expect("Topo tokens comment missing")
        .1;
    let light = tokens(topo, ":root");
    let dark = tokens(topo, ":root[data-theme=\"dark\"]");
    assert_eq!(
        light.keys().collect::<Vec<_>>(),
        dark.keys().collect::<Vec<_>>(),
        "Topo light/dark custom property mismatch: both blocks must define identical properties"
    );
    let mut output = String::from("#[derive(Copy, Clone, Debug)]\npub struct Theme {\n");
    for key in light.keys() {
        output.push_str(&format!("pub {key}: gpui::Hsla,\n"));
    }
    output.push_str("}\nimpl Theme {\n");
    for (name, values) in [("light", light), ("dark", dark)] {
        output.push_str(&format!("pub fn {name}() -> Self {{ Self {{\n"));
        for (key, value) in values {
            output.push_str(&format!("{key}: {},\n", color(&value)));
        }
        output.push_str("} }\n");
    }
    output.push_str("}\n");
    let destination =
        PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is missing")).join("theme.rs");
    fs::write(destination, output).expect("cannot write generated Topo theme");
}
