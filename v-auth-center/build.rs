use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let out_dir = env::var("OUT_DIR").unwrap();
    let api_root = Path::new(&manifest_dir).join("src").join("api");
    let model_root = Path::new(&manifest_dir).join("src").join("model");

    println!("cargo:rerun-if-changed=src/api");
    println!("cargo:rerun-if-changed=src/model");

    let mut entries: Vec<(String, PathBuf, bool)> = Vec::new();
    if api_root.exists() {
        collect_apis(&api_root, &api_root, &mut entries);
    }

    let mut code = String::new();
    code.push_str("// 自动生成的路由注册代码 / Auto-generated route registry\n");
    code.push_str("pub fn configure(cfg: &mut actix_web::web::ServiceConfig) {\n");
    code.push_str("    async fn __stub() -> impl actix_web::Responder { actix_web::HttpResponse::NotImplemented().finish() }\n");

    for (route, path, has_register) in entries.iter() {
        let abs = Path::new(&manifest_dir).join("src").join("api").join(path);
        let abs_str = abs.to_string_lossy();
        println!("cargo:rerun-if-changed=src/api/{}", escape_path(&path));
        let module_path = to_module_path(path);
        if *has_register {
            code.push_str(&format!(
                "    {}::register(cfg, \"{}\");\n",
                module_path, route
            ));
        } else {
            code.push_str(&format!(
                "    cfg.service(actix_web::web::resource(\"{}\").to(__stub));\n",
                route
            ));
            code.push_str(&format!(
                "    println!(\"warn: api at {} has no register(), using stub\");\n",
                abs_str
            ));
        }
    }

    code.push_str("}\n");

    let out_path = Path::new(&out_dir).join("api_registry.rs");
    let mut f = fs::File::create(out_path).unwrap();
    let mut rows: Vec<(String, String, String, String, String)> = Vec::new();
    for (route, path, _has_register) in entries.iter() {
        let module_path = to_module_path(path);
        let file_rel = escape_path(path);
        let abs = Path::new(&manifest_dir).join("src").join("api").join(path);
        let file_str = fs::read_to_string(&abs).unwrap_or_default();
        let methods = detect_methods(&file_str);
        let handlers = detect_handlers(&file_str);
        let resources = detect_resource_paths(&file_str);
        let method_str = if methods.is_empty() {
            "ALL".to_string()
        } else {
            methods.join("|")
        };
        let handler_str = if handlers.is_empty() {
            module_path.clone()
        } else {
            handlers.join(",")
        };
        if resources.is_empty() {
            rows.push((
                route.clone(),
                method_str.clone(),
                handler_str.clone(),
                module_path.clone(),
                file_rel.clone(),
            ));
        } else {
            for r in resources {
                rows.push((
                    r,
                    method_str.clone(),
                    handler_str.clone(),
                    module_path.clone(),
                    file_rel.clone(),
                ));
            }
        }
    }
    let route_w_max = rows
        .iter()
        .map(|(r, _, _, _, _)| r.len())
        .max()
        .unwrap_or(5)
        .max("ROUTE".len());
    let method_w_max = rows
        .iter()
        .map(|(_, m, _, _, _)| m.len())
        .max()
        .unwrap_or(4)
        .max("METHOD".len());
    let handler_w_max = rows
        .iter()
        .map(|(_, _, h, _, _)| h.len())
        .max()
        .unwrap_or(7)
        .max("HANDLER".len());
    let module_w_max = rows
        .iter()
        .map(|(_, _, _, m, _)| m.len())
        .max()
        .unwrap_or(6)
        .max("MODULE".len());
    let file_w_max = rows
        .iter()
        .map(|(_, _, _, _, f)| f.len())
        .max()
        .unwrap_or(4)
        .max("FILE".len());

    let mut print_code = String::new();
    print_code.push_str("pub fn print_routes(addr: &str, middlewares: &[&str]) {\n");
    print_code.push_str(&format!("    const ROUTE_W: usize = {};\n", route_w_max));
    print_code.push_str(&format!("    const METHOD_W: usize = {};\n", method_w_max));
    print_code.push_str(&format!(
        "    const HANDLER_W: usize = {};\n",
        handler_w_max
    ));
    print_code.push_str(&format!("    const MODULE_W: usize = {};\n", module_w_max));
    print_code.push_str(&format!("    const FILE_W: usize = {};\n", file_w_max));
    print_code.push_str("    let addr_w = std::cmp::max(\"ADDRESS\".len(), addr.len());\n");
    print_code.push_str("    let method_w = METHOD_W;\n");
    print_code.push_str("    let route_w = ROUTE_W;\n");
    print_code.push_str("    let handler_w = HANDLER_W;\n");
    print_code.push_str("    let module_w = MODULE_W;\n");
    print_code.push_str("    let file_w = FILE_W;\n");
    print_code.push_str("    let mw = middlewares.join(\",\");\n");
    print_code.push_str("    let mw_w = std::cmp::max(\"MIDDLEWARE\".len(), mw.len());\n");
    print_code.push_str("    println!(\"| {:<addr_w$} | {:<method_w$} | {:<route_w$} | {:<handler_w$} | {:<module_w$} | {:<file_w$} | {:<mw_w$} |\", \"ADDRESS\", \"METHOD\", \"ROUTE\", \"HANDLER\", \"MODULE\", \"FILE\", \"MIDDLEWARE\", addr_w=addr_w, method_w=method_w, route_w=route_w, handler_w=handler_w, module_w=module_w, file_w=file_w, mw_w=mw_w);\n");
    print_code.push_str("    println!(\"| {:-<addr_w$} | {:-<method_w$} | {:-<route_w$} | {:-<handler_w$} | {:-<module_w$} | {:-<file_w$} | {:-<mw_w$} |\", \"\", \"\", \"\", \"\", \"\", \"\", \"\", addr_w=addr_w, method_w=method_w, route_w=route_w, handler_w=handler_w, module_w=module_w, file_w=file_w, mw_w=mw_w);\n");
    for (route, method, handler, module, file_rel) in rows.iter() {
        let line = format!(
            "    println!(\"| {{:<addr_w$}} | {{:<method_w$}} | {{:<route_w$}} | {{:<handler_w$}} | {{:<module_w$}} | {{:<file_w$}} | {{:<mw_w$}} |\", addr, \"{}\", \"{}\", \"{}\", \"{}\", \"{}\", mw, addr_w=addr_w, method_w=method_w, route_w=route_w, handler_w=handler_w, module_w=module_w, file_w=file_w, mw_w=mw_w);\n",
            method, route, handler, module, file_rel
        );
        print_code.push_str(&line);
    }
    print_code.push_str("}\n");

    let mut all_code = String::new();
    all_code.push_str(&code);
    all_code.push_str(&print_code);
    f.write_all(all_code.as_bytes()).unwrap();

    // 生成模块树 / Generate module tree
    let mut dirs: BTreeMap<PathBuf, Vec<PathBuf>> = BTreeMap::new();
    let mut files: BTreeMap<PathBuf, Vec<PathBuf>> = BTreeMap::new();
    for (_, rel, _) in entries.iter() {
        let parent = rel.parent().unwrap_or(Path::new(""));
        dirs.entry(parent.to_path_buf()).or_default();
        files
            .entry(parent.to_path_buf())
            .or_default()
            .push(rel.clone());
        // 注册所有父目录
        let mut cur = parent.to_path_buf();
        while let Some(p) = cur.parent() {
            dirs.entry(cur.clone()).or_default();
            cur = p.to_path_buf();
            if cur.as_os_str().is_empty() {
                break;
            }
        }
        dirs.entry(PathBuf::new()).or_default();
    }

    let mut mod_code = String::new();
    mod_code.push_str("// 自动生成的控制器模块树 / Auto-generated api module tree\n");
    mod_code.push_str("pub mod api {\n");
    emit_dir(&mut mod_code, &PathBuf::new(), &dirs, &files);
    mod_code.push_str("}\n");

    let mod_out = Path::new(&out_dir).join("auto_mod.rs");
    let mut mf = fs::File::create(mod_out).unwrap();
    mf.write_all(mod_code.as_bytes()).unwrap();

    // models: generate Model trait implementations
    let mut model_files: Vec<PathBuf> = Vec::new();
    if model_root.exists() {
        collect_rs_files(&model_root, &model_root, &mut model_files);
    }
    let mut model_code = String::new();
    model_code.push_str("// auto-generated model impls\n");
    for rel in model_files.iter() {
        println!("cargo:rerun-if-changed=src/model/{}", escape_path(rel));
        let abs = Path::new(&manifest_dir).join("src").join("model").join(rel);
        if let Some(mc) = gen_model_impl(&abs, rel) {
            model_code.push_str(&mc);
        }
    }
    let model_out = Path::new(&out_dir).join("model_auto.rs");
    let mut mf2 = fs::File::create(model_out).unwrap();
    mf2.write_all(model_code.as_bytes()).unwrap();
}

fn collect_apis(root: &Path, base: &Path, entries: &mut Vec<(String, PathBuf, bool)>) {
    let read_dir = match fs::read_dir(root) {
        Ok(d) => d,
        Err(_) => return,
    };
    for entry in read_dir {
        if let Ok(e) = entry {
            let p = e.path();
            if p.is_dir() {
                collect_apis(&p, base, entries)
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

fn emit_dir(
    out: &mut String,
    dir: &PathBuf,
    dirs: &BTreeMap<PathBuf, Vec<PathBuf>>,
    files: &BTreeMap<PathBuf, Vec<PathBuf>>,
) {
    // 子目录 / child directories
    let mut children_dirs: Vec<PathBuf> = Vec::new();
    for k in dirs.keys() {
        if k.parent() == Some(dir.as_path()) {
            children_dirs.push(k.clone());
        }
    }
    // 子文件 / child files
    let mut children_files = files.get(dir).cloned().unwrap_or_default();
    children_dirs.sort();
    children_files.sort();

    for d in children_dirs {
        if let Some(name) = d.file_name().and_then(|s| s.to_str()) {
            out.push_str(&format!("    pub mod {} {{\n", name));
            emit_dir(out, &d, dirs, files);
            out.push_str("    }\n");
        }
    }

    for f in children_files {
        if let Some(stem) = f.file_stem().and_then(|s| s.to_str()) {
            let p = format!("/src/api/{}", escape_path(&f));
            out.push_str(&format!(
                "    pub mod {} {{ include!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"{}\")); }}\n",
                stem, p
            ));
        }
    }
}

fn to_module_path(p: &PathBuf) -> String {
    let mut parts: Vec<String> = vec!["v_auth_center".to_string(), "api".to_string()];
    for c in p.components() {
        if let std::path::Component::Normal(os) = c {
            let s = os.to_string_lossy().replace(".rs", "");
            parts.push(s);
        }
    }
    parts.join("::")
}
fn collect_rs_files(root: &Path, base: &Path, out: &mut Vec<PathBuf>) {
    let read_dir = match fs::read_dir(root) {
        Ok(d) => d,
        Err(_) => return,
    };
    for entry in read_dir {
        if let Ok(e) = entry {
            let p = e.path();
            if p.is_dir() {
                collect_rs_files(&p, base, out);
            } else if is_rs(&p) {
                if p.file_name().and_then(|s| s.to_str()) == Some("mod.rs") {
                    continue;
                }
                let rel = p.strip_prefix(base).unwrap().to_path_buf();
                out.push(rel);
            }
        }
    }
}

fn read_file(p: &Path) -> Option<String> {
    fs::read_to_string(p).ok()
}

fn gen_model_impl(abs: &Path, rel: &PathBuf) -> Option<String> {
    let s = read_file(abs)?;
    let name = extract_struct_name(&s)?;
    let table = extract_const(&s, "TABLE_NAME").unwrap_or_else(|| default_table_name(rel));
    let group = extract_const(&s, "TABLE_GROUP").unwrap_or_else(|| "default".to_string());
    let mod_path = to_model_module_path(rel);
    Some(format!(
        "impl v::db::database::Model for {mod_path}::{name} {{\n    fn table_name() -> &'static str {{ \"{table}\" }}\n    fn group_name() -> &'static str {{ \"{group}\" }}\n}}\n",
        mod_path = mod_path, name = name, table = table, group = group
    ))
}

fn extract_struct_name(s: &str) -> Option<String> {
    let pat = "pub struct ";
    if let Some(i) = s.find(pat) {
        let tail = &s[i + pat.len()..];
        let mut name = String::new();
        for ch in tail.chars() {
            if ch.is_alphanumeric() || ch == '_' {
                name.push(ch);
            } else {
                break;
            }
        }
        if !name.is_empty() {
            return Some(name);
        }
    }
    None
}

fn extract_const(s: &str, key: &str) -> Option<String> {
    let pat = format!("pub const {}: &str =", key);
    if let Some(i) = s.find(&pat) {
        let tail = &s[i + pat.len()..];
        if let Some(j) = tail.find('"') {
            let tail2 = &tail[j + 1..];
            if let Some(k) = tail2.find('"') {
                return Some(tail2[..k].to_string());
            }
        }
    }
    None
}

fn default_table_name(rel: &PathBuf) -> String {
    let stem = rel.file_stem().and_then(|s| s.to_str()).unwrap_or("model");
    format!("{}s", stem)
}

fn to_model_module_path(p: &PathBuf) -> String {
    let mut parts: Vec<String> = vec!["crate".to_string(), "model".to_string()];
    for c in p.components() {
        if let std::path::Component::Normal(os) = c {
            let s = os.to_string_lossy().replace(".rs", "");
            parts.push(s);
        }
    }
    parts.join("::")
}
fn detect_methods(s: &str) -> Vec<String> {
    let mut m = Vec::new();
    let pairs = vec![
        ("web::get()", "GET"),
        ("web::post()", "POST"),
        ("web::put()", "PUT"),
        ("web::delete()", "DELETE"),
        ("web::patch()", "PATCH"),
    ];
    for (needle, name) in pairs {
        if s.contains(needle) {
            m.push(name.to_string());
        }
    }
    m
}

fn detect_handlers(s: &str) -> Vec<String> {
    let mut hs = Vec::new();
    let key = ".to(";
    let mut start = 0usize;
    while let Some(i) = s[start..].find(key) {
        let abs_i = start + i + key.len();
        let tail = &s[abs_i..];
        let mut name = String::new();
        for ch in tail.chars() {
            if ch.is_alphanumeric() || ch == '_' {
                name.push(ch);
            } else {
                break;
            }
        }
        let nlen = name.len();
        if !name.is_empty() {
            hs.push(name);
        }
        start = abs_i + nlen;
    }
    hs
}

fn detect_resource_paths(s: &str) -> Vec<String> {
    let mut rs = Vec::new();
    let key = "web::resource(\"";
    let mut start = 0usize;
    while let Some(i) = s[start..].find(key) {
        let abs_i = start + i + key.len();
        let tail = &s[abs_i..];
        let mut path = String::new();
        for ch in tail.chars() {
            if ch == '"' {
                break;
            }
            path.push(ch);
        }
        let plen = path.len();
        if !path.is_empty() {
            rs.push(path);
        }
        start = abs_i + plen;
    }
    rs
}
