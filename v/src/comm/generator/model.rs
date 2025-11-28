use crate::comm::generator::GenError;
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::result::Result;

pub fn generate_model_artifacts(manifest_dir: &str, out_dir: &str) -> Result<(), GenError> {
    let model_root = Path::new(manifest_dir).join("src").join("model");
    let mut model_files: Vec<PathBuf> = Vec::new();
    if model_root.exists() {
        collect_rs_files(&model_root, &model_root, &mut model_files);
    }

    let mut model_code = String::new();
    model_code.push_str("// Auto-generated model impls / 自动生成的模型接口实现\n");
    for rel in model_files.iter() {
        println!("cargo:rerun-if-changed=src/model/{}", escape_path(rel));
        let abs = Path::new(manifest_dir).join("src").join("model").join(rel);
        if let Some(mc) = gen_model_impl(&abs, rel) {
            model_code.push_str(&mc);
        }
    }
    let model_out = Path::new(out_dir).join("model_auto.rs");
    let mut mf2 = fs::File::create(model_out)?;
    mf2.write_all(model_code.as_bytes())?;

    let mut m_dirs: BTreeMap<PathBuf, Vec<PathBuf>> = BTreeMap::new();
    let mut m_files: BTreeMap<PathBuf, Vec<PathBuf>> = BTreeMap::new();
    for rel in model_files.iter() {
        let parent = rel.parent().unwrap_or(Path::new(""));
        m_dirs.entry(parent.to_path_buf()).or_default();
        m_files
            .entry(parent.to_path_buf())
            .or_default()
            .push(rel.clone());
        let mut cur = parent.to_path_buf();
        while let Some(p) = cur.parent() {
            m_dirs.entry(cur.clone()).or_default();
            cur = p.to_path_buf();
            if cur.as_os_str().is_empty() {
                break;
            }
        }
        m_dirs.entry(PathBuf::new()).or_default();
    }

    let mut model_tree_code = String::new();
    model_tree_code.push_str("// Auto-generated model module tree / 自动生成的模型模块树\n");
    emit_model_dir(&mut model_tree_code, &PathBuf::new(), &m_dirs, &m_files);
    let model_tree_out = Path::new(out_dir).join("model_tree.rs");
    let mut mf3 = fs::File::create(model_tree_out)?;
    mf3.write_all(model_tree_code.as_bytes())?;
    Ok(())
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

fn is_rs(p: &Path) -> bool {
    matches!(p.extension().and_then(|s| s.to_str()), Some("rs"))
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
        "impl v::db::model::DbModel for {mod_path}::{name} {{\n    fn table_name() -> &'static str {{ \"{table}\" }}\n    fn table_group() -> &'static str {{ \"{group}\" }}\n}}\n",
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
            parts.push(os.to_string_lossy().replace(".rs", ""));
        }
    }
    parts.join("::")
}

fn escape_path(p: &PathBuf) -> String {
    p.to_string_lossy().replace("\\", "/")
}

fn emit_model_dir(
    out: &mut String,
    dir: &PathBuf,
    dirs: &BTreeMap<PathBuf, Vec<PathBuf>>,
    files: &BTreeMap<PathBuf, Vec<PathBuf>>,
) {
    let mut children_dirs: Vec<PathBuf> = Vec::new();
    for k in dirs.keys() {
        if k.parent() == Some(dir.as_path()) {
            children_dirs.push(k.clone());
        }
    }
    let mut children_files = files.get(dir).cloned().unwrap_or_default();
    children_dirs.sort();
    children_files.sort();
    for d in children_dirs {
        if let Some(name) = d.file_name().and_then(|s| s.to_str()) {
            out.push_str(&format!("pub mod {} {{\n", name));
            emit_model_dir(out, &d, dirs, files);
            out.push_str("}\n");
        }
    }
    for f in children_files {
        if let Some(stem) = f.file_stem().and_then(|s| s.to_str()) {
            let p = format!("/src/model/{}", escape_path(&f));
            out.push_str(&format!(
                "pub mod {} {{ include!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"{}\")); }}\n",
                stem, p
            ));
        }
    }
}
