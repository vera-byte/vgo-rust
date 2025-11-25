use crate::comm::generator::GenError;
use std::result::Result;
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

pub fn generate_api_artifacts(manifest_dir: &str, out_dir: &str) -> Result<(), GenError> {
    let crate_root = std::env::var("CARGO_PKG_NAME").unwrap_or_else(|_| "crate".to_string()).replace('-', "_");
    let api_root = Path::new(manifest_dir).join("src").join("api");
    let mut entries: Vec<(String, PathBuf, bool)> = Vec::new();
    if api_root.exists() { collect_apis(&api_root, &api_root, &mut entries); }

    let mut registry_code = String::new();
    registry_code.push_str("// Auto-generated route registry / 自动生成的路由注册代码\n");
    registry_code.push_str("pub fn configure(cfg: &mut actix_web::web::ServiceConfig) {\n");
    registry_code.push_str("    async fn __stub() -> impl actix_web::Responder { actix_web::HttpResponse::NotImplemented().finish() }\n");
    for (route, path, has_register) in entries.iter() {
        let abs = Path::new(manifest_dir).join("src").join("api").join(path);
        println!("cargo:rerun-if-changed=src/api/{}", escape_path(path));
        let module_path = to_module_path(path, &crate_root);
        if *has_register { registry_code.push_str(&format!("    {}::register(cfg, \"{}\");\n", module_path, route)); }
        else {
            registry_code.push_str(&format!("    cfg.service(actix_web::web::resource(\"{}\").to(__stub));\n", route));
            registry_code.push_str(&format!("    println!(\"warn: api at {} has no register(), using stub\");\n", abs.to_string_lossy()));
        }
    }
    registry_code.push_str("}\n");

    let out_path = Path::new(out_dir).join("api_registry.rs");
    let mut f = fs::File::create(out_path)?;

    let mut rows: Vec<(String, String, String, String, String)> = Vec::new();
    for (route, path, _has_register) in entries.iter() {
        let module_path = to_module_path(path, &crate_root);
        let file_rel = escape_path(path);
        let abs = Path::new(manifest_dir).join("src").join("api").join(path);
        let file_str = fs::read_to_string(&abs).unwrap_or_default();
        let methods = detect_methods(&file_str);
        let handlers = detect_handlers(&file_str);
        let resources = detect_resource_paths(&file_str);
        let method_str = if methods.is_empty() { "ALL".to_string() } else { methods.join("|") };
        let handler_str = if handlers.is_empty() { module_path.clone() } else { handlers.join(",") };
        if resources.is_empty() { rows.push((route.clone(), method_str.clone(), handler_str.clone(), module_path.clone(), file_rel.clone())); }
        else { for r in resources { rows.push((r, method_str.clone(), handler_str.clone(), module_path.clone(), file_rel.clone())); } }
    }

    let route_w_max = rows.iter().map(|(r,_,_,_,_)| r.len()).max().unwrap_or(5).max("ROUTE".len());
    let method_w_max = rows.iter().map(|(_,m,_,_,_)| m.len()).max().unwrap_or(4).max("METHOD".len());
    let handler_w_max = rows.iter().map(|(_,_,h,_,_)| h.len()).max().unwrap_or(7).max("HANDLER".len());
    let module_w_max = rows.iter().map(|(_,_,_,m,_)| m.len()).max().unwrap_or(6).max("MODULE".len());
    let file_w_max = rows.iter().map(|(_,_,_,_,f)| f.len()).max().unwrap_or(4).max("FILE".len());

    let mut print_code = String::new();
    print_code.push_str("pub fn print_routes(addr: &str, middlewares: &[&str]) {\n");
    print_code.push_str(&format!("    const ROUTE_W: usize = {};\n", route_w_max));
    print_code.push_str(&format!("    const METHOD_W: usize = {};\n", method_w_max));
    print_code.push_str(&format!("    const HANDLER_W: usize = {};\n", handler_w_max));
    print_code.push_str(&format!("    const MODULE_W: usize = {};\n", module_w_max));
    print_code.push_str(&format!("    const FILE_W: usize = {};\n", file_w_max));
    print_code.push_str("    let addr_w = std::cmp::max(\"ADDRESS\".len(), addr.len());\n");
    print_code.push_str("    let method_w = METHOD_W;\n");
    print_code.push_str("    let _route_w = ROUTE_W;\n");
    print_code.push_str("    let _handler_w = HANDLER_W;\n");
    print_code.push_str("    let _module_w = MODULE_W;\n");
    print_code.push_str("    let _file_w = FILE_W;\n");
    print_code.push_str("    let mw = middlewares.join(\",\");\n");
    print_code.push_str("    let mw_w = std::cmp::max(\"MIDDLEWARE\".len(), mw.len());\n");
    print_code.push_str("    println!(\"| {:<addr_w$} | {:<method_w$} | {:<ROUTE_W$} | {:<HANDLER_W$} | {:<MODULE_W$} | {:<FILE_W$} | {:<mw_w$} |\", \"ADDRESS\", \"METHOD\", \"ROUTE\", \"HANDLER\", \"MODULE\", \"FILE\", \"MIDDLEWARE\");\n");
    print_code.push_str("    println!(\"| {:-<addr_w$} | {:-<method_w$} | {:-<ROUTE_W$} | {:-<HANDLER_W$} | {:-<MODULE_W$} | {:-<FILE_W$} | {:-<mw_w$} |\", \"\", \"\", \"\", \"\", \"\", \"\", \"\");\n");
    for (route, method, handler, module, file_rel) in rows.iter() {
        let line = format!("    println!(\"| {{:<addr_w$}} | {{:<method_w$}} | {{:<ROUTE_W$}} | {{:<HANDLER_W$}} | {{:<MODULE_W$}} | {{:<FILE_W$}} | {{:<mw_w$}} |\", addr, \"{}\", \"{}\", \"{}\", \"{}\", \"{}\", mw);\n", method, route, handler, module, file_rel);
        print_code.push_str(&line);
    }
    print_code.push_str("}\n");

    let mut all_code = String::new();
    all_code.push_str(&registry_code);
    all_code.push_str(&print_code);
    f.write_all(all_code.as_bytes())?;

    let mut dirs: BTreeMap<PathBuf, Vec<PathBuf>> = BTreeMap::new();
    let mut files: BTreeMap<PathBuf, Vec<PathBuf>> = BTreeMap::new();
    for (_, rel, _) in entries.iter() {
        let parent = rel.parent().unwrap_or(Path::new(""));
        dirs.entry(parent.to_path_buf()).or_default();
        files.entry(parent.to_path_buf()).or_default().push(rel.clone());
        let mut cur = parent.to_path_buf();
        while let Some(p) = cur.parent() { dirs.entry(cur.clone()).or_default(); cur = p.to_path_buf(); if cur.as_os_str().is_empty() { break; } }
        dirs.entry(PathBuf::new()).or_default();
    }
    let mut mod_code = String::new();
    mod_code.push_str("// Auto-generated api module tree / 自动生成的控制器模块树\n");
    mod_code.push_str("pub mod api {\n");
    emit_dir(&mut mod_code, &PathBuf::new(), &dirs, &files);
    mod_code.push_str("}\n");
    let mod_out = Path::new(out_dir).join("auto_mod.rs");
    let mut mf = fs::File::create(mod_out)?;
    mf.write_all(mod_code.as_bytes())?;
    Ok(())
}

fn collect_apis(root: &Path, base: &Path, entries: &mut Vec<(String, PathBuf, bool)>) {
    let read_dir = match fs::read_dir(root) { Ok(d) => d, Err(_) => return };
    for entry in read_dir { if let Ok(e) = entry { let p = e.path(); if p.is_dir() { collect_apis(&p, base, entries) } else if is_rs(&p) { if p.file_name().and_then(|s| s.to_str()) == Some("mod.rs") { continue; } let rel = p.strip_prefix(base).unwrap().to_path_buf(); let mut route = rel_to_route(&rel); let has_register = file_contains(&p, "fn register("); if let Some(alias) = extract_route_alias(&p) { route = alias; } entries.push((route, rel, has_register)); } } }
}

fn is_rs(p: &Path) -> bool { matches!(p.extension().and_then(|s| s.to_str()), Some("rs")) }

fn rel_to_route(rel: &PathBuf) -> String { let mut parts: Vec<String> = Vec::new(); for c in rel.components() { if let std::path::Component::Normal(os) = c { let s = os.to_string_lossy(); parts.push(s.replace(".rs", "")) } } format!("/{}", parts.join("/")) }

fn file_contains(p: &Path, needle: &str) -> bool { fs::read_to_string(p).map(|s| s.contains(needle)).unwrap_or(false) }

fn extract_route_alias(p: &Path) -> Option<String> { let s = fs::read_to_string(p).ok()?; let key = "pub const ROUTE_PATH: &str ="; if let Some(idx) = s.find(key) { let tail = &s[idx + key.len()..]; if let Some(start) = tail.find('"') { let tail2 = &tail[start + 1..]; if let Some(end) = tail2.find('"') { return Some(tail2[..end].to_string()); } } } None }

fn escape_path(p: &PathBuf) -> String { p.to_string_lossy().replace("\\", "/") }

fn emit_dir(out: &mut String, dir: &PathBuf, dirs: &BTreeMap<PathBuf, Vec<PathBuf>>, files: &BTreeMap<PathBuf, Vec<PathBuf>>) {
    let mut children_dirs: Vec<PathBuf> = Vec::new();
    for k in dirs.keys() { if k.parent() == Some(dir.as_path()) { children_dirs.push(k.clone()); } }
    let mut children_files = files.get(dir).cloned().unwrap_or_default();
    children_dirs.sort(); children_files.sort();
    for d in children_dirs { if let Some(name) = d.file_name().and_then(|s| s.to_str()) { out.push_str(&format!("    pub mod {} {{\n", name)); emit_dir(out, &d, dirs, files); out.push_str("    }\n"); } }
    for f in children_files { if let Some(stem) = f.file_stem().and_then(|s| s.to_str()) { let p = format!("/src/api/{}", escape_path(&f)); out.push_str(&format!("    pub mod {} {{ include!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"{}\")); }}\n", stem, p)); } }
}

fn to_module_path(p: &PathBuf, crate_root: &str) -> String { let mut parts: Vec<String> = vec![crate_root.to_string(), "api".to_string()]; for c in p.components() { if let std::path::Component::Normal(os) = c { parts.push(os.to_string_lossy().replace(".rs", "")); } } parts.join("::") }

fn detect_methods(s: &str) -> Vec<String> { let mut m = Vec::new(); for (needle, name) in [("web::get()", "GET"), ("web::post()", "POST"), ("web::put()", "PUT"), ("web::delete()", "DELETE"), ("web::patch()", "PATCH")] { if s.contains(needle) { m.push(name.to_string()); } } m }

fn detect_handlers(s: &str) -> Vec<String> { let mut hs = Vec::new(); let key = ".to("; let mut start = 0usize; while let Some(i) = s[start..].find(key) { let abs_i = start + i + key.len(); let tail = &s[abs_i..]; let mut name = String::new(); for ch in tail.chars() { if ch.is_alphanumeric() || ch == '_' { name.push(ch); } else { break; } } let nlen = name.len(); if !name.is_empty() { hs.push(name); } start = abs_i + nlen; } hs }

fn detect_resource_paths(s: &str) -> Vec<String> { let mut rs = Vec::new(); let key = "web::resource(\""; let mut start = 0usize; while let Some(i) = s[start..].find(key) { let abs_i = start + i + key.len(); let tail = &s[abs_i..]; let mut path = String::new(); for ch in tail.chars() { if ch == '"' { break; } path.push(ch); } let plen = path.len(); if !path.is_empty() { rs.push(path); } start = abs_i + plen; } rs }
