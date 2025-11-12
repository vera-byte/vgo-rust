use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let out_dir = env::var("OUT_DIR").unwrap();
    let controller_root = Path::new(&manifest_dir).join("src").join("controller");

    println!("cargo:rerun-if-changed=src/controller");

    let mut entries: Vec<(String, PathBuf, bool)> = Vec::new();
    if controller_root.exists() {
        collect_controllers(&controller_root, &controller_root, &mut entries);
    }

    let mut code = String::new();
    code.push_str("pub fn configure(cfg: &mut actix_web::web::ServiceConfig) {\n");
    code.push_str("    async fn __stub() -> impl actix_web::Responder { actix_web::HttpResponse::NotImplemented().finish() }\n");

    for (route, path, has_register) in entries.iter() {
        let module_name = sanitize_module_name(path);
        let abs = Path::new(&manifest_dir)
            .join("src")
            .join("controller")
            .join(path);
        let abs_str = abs.to_string_lossy();
        println!(
            "cargo:rerun-if-changed=src/controller/{}",
            escape_path(&path)
        );
        code.push_str(&format!("    pub mod {} {{ include!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/src/controller/{}\")); }}\n", module_name, escape_path(&path)));
        if *has_register {
            code.push_str(&format!(
                "    {}::register(cfg, \"{}\");\n",
                module_name, route
            ));
        } else {
            code.push_str(&format!(
                "    cfg.service(actix_web::web::resource(\"{}\").to(__stub));\n",
                route
            ));
            code.push_str(&format!(
                "    println!(\"warn: controller at {} has no register(), using stub\");\n",
                abs_str
            ));
        }
    }

    code.push_str("}\n");

    let out_path = Path::new(&out_dir).join("controller_registry.rs");
    let mut f = fs::File::create(out_path).unwrap();
    f.write_all(code.as_bytes()).unwrap();
}

fn collect_controllers(root: &Path, base: &Path, entries: &mut Vec<(String, PathBuf, bool)>) {
    let read_dir = match fs::read_dir(root) {
        Ok(d) => d,
        Err(_) => return,
    };
    for entry in read_dir {
        if let Ok(e) = entry {
            let p = e.path();
            if p.is_dir() {
                collect_controllers(&p, base, entries)
            } else if is_rs(&p) {
                if p.file_name().and_then(|s| s.to_str()) == Some("mod.rs") {
                    continue;
                }
                let rel = p.strip_prefix(base).unwrap().to_path_buf();
                let mut route = rel_to_route(&rel);
                let has_register = file_contains(&p, "fn register(");
                if let Some(alias) = extract_route_alias(&p) {
                    route = alias;
                }
                entries.push((route, rel, has_register));
            }
        }
    }
}

fn is_rs(p: &Path) -> bool {
    matches!(p.extension().and_then(|s| s.to_str()), Some("rs"))
}

fn rel_to_route(rel: &PathBuf) -> String {
    let mut parts: Vec<String> = Vec::new();
    for c in rel.components() {
        match c {
            std::path::Component::Normal(os) => {
                let s = os.to_string_lossy();
                let t = s.replace(".rs", "");
                parts.push(t)
            }
            _ => {}
        }
    }
    format!("/{}", parts.join("/"))
}

fn file_contains(p: &Path, needle: &str) -> bool {
    match fs::read_to_string(p) {
        Ok(s) => s.contains(needle),
        Err(_) => false,
    }
}

fn extract_route_alias(p: &Path) -> Option<String> {
    let s = fs::read_to_string(p).ok()?;
    let key = "pub const ROUTE_PATH: &str =";
    if let Some(idx) = s.find(key) {
        let tail = &s[idx + key.len()..];
        if let Some(start) = tail.find('"') {
            let tail2 = &tail[start + 1..];
            if let Some(end) = tail2.find('"') {
                let v = &tail2[..end];
                return Some(v.to_string());
            }
        }
    }
    None
}

fn sanitize_module_name(p: &PathBuf) -> String {
    let s = p.to_string_lossy();
    let mut out = String::from("ctrl_");
    for ch in s.chars() {
        match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' => out.push(ch),
            _ => out.push('_'),
        }
    }
    out
}

fn escape_path(p: &PathBuf) -> String {
    p.to_string_lossy().replace("\\", "/")
}
