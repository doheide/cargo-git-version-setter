use std::collections::HashMap;
use std::fmt::Display;
use std::fs;
use std::path::PathBuf;
use std::process::exit;
use clap::ValueEnum;
use console::{style, Emoji};
use regex::Regex;
use toml_edit::DocumentMut;

// ********************************************************
// ********************************************************
pub static LOOKING_GLASS: Emoji<'_, '_> = Emoji("🔍", "");
pub static TRUCK: Emoji<'_, '_> = Emoji("🚚", "");
pub static CLIP: Emoji<'_, '_> = Emoji("🔗", "");
pub static PEN: Emoji<'_, '_> = Emoji("🖊️", "");
pub static TAG: Emoji<'_, '_> = Emoji("🏷️", "");
pub static CHECK: Emoji<'_, '_> = Emoji("✔ ", "");
pub static INDENT: &str = "       ";


// ********************************************************
// ********************************************************
#[derive(ValueEnum, Clone)]
pub enum CargoFile {
    /// Write version to leaf cargo file
    Leaf,
    /// Write version to base cargo file
    Base,
    /// Write version to all cargo files
    All
}

#[derive(ValueEnum, Clone, PartialEq, Debug)]
pub enum IncrementVersionPart {
    /// Patch version when you make backward compatible bug fixes
    Patch,
    /// Minor version when you add functionality in a backward compatible manner
    Minor,
    /// Major version when you make incompatible API changes
    Major
}
impl Display for IncrementVersionPart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            IncrementVersionPart::Patch => { "patch".to_string() }
            IncrementVersionPart::Minor => { "minor".to_string() }
            IncrementVersionPart::Major => { "major".to_string() }
        };
        write!(f, "{}", str)
    }
}

#[derive(PartialEq, Clone)]
pub struct Version {
    major: u16,
    minor: u16,
    patch: u16,
}
impl Version {
    pub fn increment(&mut self, part: &IncrementVersionPart) {
        match part {
            IncrementVersionPart::Patch => {
                self.patch += 1 }
            IncrementVersionPart::Minor => {
                self.minor += 1; self.patch = 0; }
            IncrementVersionPart::Major => {
                self.major += 1; self.minor = 0; self.patch = 0;
            }
        }
    }
    pub fn increment_clone(&self, part: &IncrementVersionPart) -> Self {
        let mut n = self.clone();
        n.increment(part);
        n
    }
}
impl TryFrom<String> for Version {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let re = Regex::new(r"([0-9]+)\.([0-9]+)\.([0-9]+)").unwrap();

        let rea = match re.captures(value.as_str()) {
            None => return Err("Invalid version string"), Some(v) => v
        };

        let major = rea.get(1).unwrap().as_str().parse::<u16>().unwrap_or(0);
        let minor = rea.get(2).unwrap().as_str().parse::<u16>().unwrap_or(0);
        let patch = rea.get(3).unwrap().as_str().parse::<u16>().unwrap_or(0);

        Ok(Self { major, minor, patch })
    }
}
impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", format!("{}.{}.{}", self.major, self.minor, self.patch))
    }
}
// ********************************************************
// ********************************************************
pub fn print_error(msg: String) -> ! {
    println!("\n{} {}", style("Error:").bold().red(), msg);
    exit(-1);
}
#[allow(dead_code)]
pub fn print_warn(msg: String) {
    println!("\n{} {}", style("Warning:").bold().yellow(), msg);
}

// ********************************************************
// ********************************************************
pub fn filter_cargo_tomls_by_selector(cargo_tomls: Vec<PathBuf>, cargo_file_selector: &Option<CargoFile>) -> Vec<PathBuf> {
    match cargo_file_selector {
        None => {
            if cargo_tomls.len() > 1 { print_error("Multiple cargo files found but option cargo_file_selector not set".into()); }
            else { cargo_tomls }
        },
        Some(cfs) => match cfs{
            CargoFile::Leaf => {
                let cct = cargo_tomls.iter().fold((0, PathBuf::new()), |(max_len, pb), cpb| {
                    let cl = cpb.as_os_str().len();
                    if max_len < cl { (cl, cpb.clone()) } else { (max_len, pb) }
                }).1;
                println!("{INDENT}  -> using leaf: {}", cct.display());
                vec![cct]
            },
            CargoFile::Base => {
                let cct = cargo_tomls.iter().fold((usize::MAX, PathBuf::new()), |(max_len, pb), cpb| {
                    let cl = cpb.as_os_str().len();
                    if max_len > cl { (cl, cpb.clone()) } else { (max_len, pb) }
                }).1;
                println!("{INDENT}  -> using base: {}", cct.display());
                vec![cct]
            },
            CargoFile::All => {
                println!("{INDENT}  -> using all.");
                cargo_tomls }
        }
    }
}

pub fn find_cargo_tomls_and_git_base(path: PathBuf, scan_subdirs: bool) -> (Vec<PathBuf>, Option<PathBuf>){
    let mut ct: Vec<PathBuf> = vec![];
    let mut cp = path.clone();

    let git_base_dir = loop {
        cp.push("Cargo.toml");
        if cp.exists() && cp.is_file() {
            ct.push(cp.clone());
        }
        cp.pop();

        cp.push(".git");
        if cp.exists() && cp.is_dir() {
            break Some(cp.parent().unwrap().to_path_buf());
        }
        cp.pop();

        cp = match cp.parent() {
            Some(parent) => parent.to_path_buf(),
            None => { break None; }
        }
    };

    fn read_dir_cargos(dir: PathBuf, only_subdirs: bool) -> Vec<PathBuf> {
        let mut cv = vec![];
        for f in fs::read_dir(dir).unwrap() {
            let p = f.unwrap().path();
            if p.is_dir()  {
                let v = read_dir_cargos(p, true);
                cv.extend(v);
            }
            else if p.file_name().unwrap().to_str().unwrap() == "Cargo.toml" && only_subdirs {
                cv.push(p);
            }
        }
        cv
    }
    if scan_subdirs {
        ct.extend(read_dir_cargos(path.clone(), false));
    }
    (ct, git_base_dir)
}

pub fn read_version_tomls(cargo_tomls: &Vec<PathBuf>) -> HashMap<PathBuf, (Version, DocumentMut)> {
    let mut cargo_content = HashMap::<PathBuf, (Version, DocumentMut)>::new();
    for cct in cargo_tomls {
        let cct_content = match fs::read(cct.clone()) {
            Ok(content) => String::from_utf8(content).unwrap(),
            Err(e) => { print_error(format!("Could not read file '{}': {:?}", cct.display(), e)); }
        };
        let toml = match cct_content.parse::<DocumentMut>() {
            Ok(v) => v, Err(e) => {
                print_error(format!("Could not parse toml form file '{}': {:?}", cct.display(), e)); }
        };

        match Version::try_from(toml["package"]["version"].clone().to_string()) {
            Ok(v) => { cargo_content.insert(cct.clone(), (v, toml)); },
            Err(e) => { print_error(format!("Could not parse version from toml file '{}': {:?}", cct.display(), e)); }
        }
    }
    cargo_content
}

// ********************************************************
// ********************************************************
#[cfg(test)]
mod tests_filter {
    use super::*;

    #[test]
    fn test_filter_cargo_tomls_by_selector_all() {
        let tomls_simu = vec![PathBuf::from("base"), PathBuf::from("middle"), PathBuf::from("longlonglong")];

        let r = filter_cargo_tomls_by_selector(tomls_simu.clone(), &Some(CargoFile::All));
        assert_eq!(r.len(), 3);
        assert_eq!(r[0], tomls_simu[0].clone());
        assert_eq!(r[1], tomls_simu[1].clone());
        assert_eq!(r[2], tomls_simu[2].clone());
    }
    #[test]
    fn test_filter_cargo_tomls_by_selector_base() {
        let tomls_simu = vec![PathBuf::from("base"), PathBuf::from("middle"), PathBuf::from("longlonglong")];

        let r = filter_cargo_tomls_by_selector(tomls_simu.clone(), &Some(CargoFile::Base));
        assert_eq!(r.len(), 1);
        assert_eq!(r[0], tomls_simu[0].clone());
    }
    #[test]
    fn test_filter_cargo_tomls_by_selector_leaf() {
        let tomls_simu = vec![PathBuf::from("base"), PathBuf::from("middle"), PathBuf::from("longlonglong")];

        let r = filter_cargo_tomls_by_selector(tomls_simu.clone(), &Some(CargoFile::Leaf));
        assert_eq!(r.len(), 1);
        assert_eq!(r[0], tomls_simu[2].clone());
    }
}

#[cfg(test)]
mod tests_find {
    use super::*;

    #[test]
    fn test_find_cargo_tomls_without_subdirs() {
        if !PathBuf::from("./test_data").exists() {
            panic!("Test needs to be exeecuted in base dir");
        }

        let (cargo_tomls, git_base_path) = find_cargo_tomls_and_git_base(PathBuf::from("./test_data"), false);
        assert_eq!(git_base_path, Some(PathBuf::from("./")));
        assert_eq!(cargo_tomls.len(), 1);
        assert_eq!(cargo_tomls[0], PathBuf::from("./Cargo.toml"));
    }
    #[test]
    fn test_find_cargo_tomls_with_subdirs() {
        if !PathBuf::from("./test_data").exists() {
            panic!("Test needs to be exeecuted in base dir");
        }

        let (cargo_tomls, git_base_path) = find_cargo_tomls_and_git_base(PathBuf::from("./test_data"), true);
        let mut cargo_tomls_sorted = cargo_tomls.clone();
        cargo_tomls_sorted.sort_by(|a, b| {
            let a_s = a.display().to_string();
            let b_s = b.display().to_string();
            a_s.cmp(&b_s)
        });

        assert_eq!(git_base_path, Some(PathBuf::from("./")));
        assert_eq!(cargo_tomls_sorted.len(), 3);
        assert_eq!(cargo_tomls_sorted[0], PathBuf::from("./Cargo.toml"));
        assert_eq!(cargo_tomls_sorted[1], PathBuf::from("./test_data/with_different_ver/Cargo.toml"));
        assert_eq!(cargo_tomls_sorted[2], PathBuf::from("./test_data/with_same_ver/Cargo.toml"));
    }
}