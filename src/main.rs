mod utils;

use utils::*;

use std::path::PathBuf;
use std::{thread};
use std::fs::write;
use clap::{Parser, Subcommand,};
use std::time::Duration;
use toml_edit::{value};
use git2::{Repository, StatusOptions};
use git2_credentials::CredentialHandler;
use pathdiff::diff_paths;


/// Simple program to greet a person
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Path of the project
    #[arg(short, long)]
    path: Option<PathBuf>,

    /// Select cargo file, if multiple
    #[arg(short, long)]
    cargo_file_selector: Option<CargoFile>,

    /// Scan subdirectories for cargo.toml files
    #[arg(short, long, default_value_t = false)]
    scan_subdirs: bool,

    /// Do execute 'git push' and 'git push --tags'
    #[arg(short, long, default_value_t = true)]
    do_push: bool,

    /// Turn debugging information on
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Message when adding the tag to git
    #[arg(short, long)]
    tag_message: String,

    /// git remote name to push new commits to. Defaults to 'origin' if not set
    #[arg(short, long)]
    remote: Option<String>,

    /// Prefix for the version tag, defaults to 'v'
    #[arg(short, long)]
    git_prefix_for_tag: Option<String>,

    #[command(subcommand)]
    change_type: VersionChangeType,
}

#[derive(Subcommand, PartialEq, Debug)]
enum VersionChangeType {
    /// Set fixed version
    Fixed {
        // #[arg(short, long)]
        full_version: String
    },
    /// Increment part of the version. When incrementing major or minor version parts,
    /// the lower version parts are set to zero
    Increment {
        // #[arg(short, long)]
        vtype: IncrementVersionPart,
    },
    /// Only show versions from cargo and git
    OnlyShow
}



fn main() {
    let cli = Cli::parse();

    let path = {
        let p = cli.path.unwrap_or_else(|| PathBuf::from("./"));
        if p.is_file() { p.parent().unwrap().to_path_buf() }
        else { p }
    };
    if !path.exists() { print_error(format!("Path does not exist ({})", path.display())); }
    if !path.is_dir() { print_error(format!("Path is not a directory ({})", path.display())); }

    if cli.verbose > 0 { println!("Using path: {}", path.display()); }


    // ***
    let txt = String::from("Analysing cargo project");
    println!("[1/5] {} {} ...", LOOKING_GLASS, txt);

    let (cargo_tomls, git_base_path) = find_cargo_tomls_and_git_base(path, cli.scan_subdirs);
    if cargo_tomls.is_empty() { print_error("No cargo.toml found.".to_string()); }

    let git_base_path = match git_base_path {
        Some(path) => path, None => { print_error("Could not find git base path.".to_string()) }
    };
    println!("{INDENT}Found git base path: {}", git_base_path.display());
    println!("{INDENT}Found cargo.toml:\n{INDENT} - {}", cargo_tomls.iter().map(|ct| {
        ct.display().to_string() }).collect::<Vec<String>>().join(format!("\n{INDENT} - ").as_str()));

    let cargo_tomls = filter_cargo_tomls_by_selector(cargo_tomls, &cli.cargo_file_selector);

    // Init git repo and remote
    println!("{INDENT}Opening git repo ...");
    let repo = match Repository::open(git_base_path.clone()) {
        Ok(repo) => repo,
        Err(e) => panic!("failed to open git repo: {}", e),
    };
    if repo.is_bare() {
        print_error("Cannot use bare repository".to_string());
    }

    let mut git_remote = {
        let git_remote_name = match cli.remote {
            None => {
                if cli.verbose > 0 { println!("Setting git remote to 'origin' as it was not specified"); }
                "origin".to_string()
            }
            Some(r) => r
        };

        match repo.find_remote(&git_remote_name) {
            Ok(r) => r,
            Err(e) => print_error(format!("Failed to find git remote '{}' with error {}", git_remote_name, e)),
        }
    };
    println!("{INDENT}Found remote to be used: {}", git_remote.name().unwrap());

    let mut cb = git2::RemoteCallbacks::new();
    let git_config = repo.config().unwrap();
    let mut ch = CredentialHandler::new(git_config);
    cb.credentials(move |url, username, allowed| ch.try_next_credential(url, username, allowed));
    let mut po = git2::PushOptions::new();
    po.remote_callbacks(cb);

    println!("       {} {} done", CHECK, txt);

    // ***
    let txt = String::from("Writing version to cargo.toml(s)");
    println!("[2/5] {} {} ...", PEN, txt);

    let mut cargo_content = read_version_tomls(&cargo_tomls);

    // Check if version
    if cargo_content.len() > 1 && cli.cargo_file_selector.is_none() {
        print_error("More than one cargo.toml found but option cargo_file_selector not given".to_string());
    }

    let new_version = match &cli.change_type {
        VersionChangeType::Increment{ vtype } => {
            // test if all versions are equal (should work also with one cargo.toml
            let (version_to_test_against, _) = cargo_content.get(cargo_content.keys().next().unwrap()).unwrap();

            let all_versions_equal = cargo_content.iter().fold(true, |acc, (_, (cv, _))| {
                let e = version_to_test_against == cv;
                acc && e });
            if !all_versions_equal {
                match cli.cargo_file_selector.unwrap() {
                    CargoFile::All => { print_error(
                        "When using increment and updating all cargo-toml files, the versions have to be equal in all files. Use fixed in this case ...".to_string()); },
                    _ => ()
                }
            }
            version_to_test_against.increment_clone(vtype)
        },
        VersionChangeType::Fixed { full_version } => {
            match Version::try_from(full_version.clone()) {
                Ok(version) => version,
                Err(_) => { print_error(format!("Wrong format for version specifier '{}'.", full_version)) }
            }
        }
        VersionChangeType::OnlyShow => {
            print_error("Not yet implemented!!!".to_string());
        }
    };

    println!("{INDENT}New version to be written: {}", new_version.to_string());

    // ****************************************
    let mut opts = StatusOptions::new();
    opts.include_untracked(false);
    let change_count = repo.statuses(None).unwrap().iter().count();
    if change_count > 0 {
        print_error(format!("There are {} uncommitted changes - please commit before continuing.", change_count));
    }

    let git_tag_prefix = cli.git_prefix_for_tag.unwrap_or("v".to_string());
    let git_tag_new_version_str = format!("{git_tag_prefix}{}", new_version.to_string());
    let tns = repo.tag_names(Some(format!("{git_tag_prefix}*").as_str())).unwrap()
        .into_iter().filter_map(|ct| { match ct {
            None => None,
            Some(s) => Some(String::from(s))
        } }).collect::<Vec<_>>();
    if tns.contains(&git_tag_new_version_str) {
        print_error(format!("New version already exists as git tag '{}' -> Aborting", git_tag_new_version_str));
    }

    //
    cargo_content.iter_mut().for_each(|(fname, (_, toml))| {
        toml["package"]["version"] = value(new_version.to_string());
        // println!("file: {}\ntoml: {}", fname.display(), toml.to_string());
        if let Err(e) = write(fname, toml.to_string()) {
            print_error(format!("Failed to write to '{}': {}", fname.display(), e));
        }
    });

    println!("       {} {} done", CHECK, txt);

    // ***
    let txt = String::from("git commit for cargo.toml(s)");
    println!("[3/5] {} {} ...", CLIP, txt);

    // https://users.rust-lang.org/t/how-can-i-do-git-add-some-file-rs-git-commit-m-message-git-push-with-git2-crate-on-a-bare-repo/94109/3
    // open the index database of the given repository
    // the repo can't be bare, must have a worktree
    let mut index = repo.index().unwrap();
    // suppose you made some change to "hello.txt", add it to the index
    cargo_content.keys().into_iter().for_each(|fname| {
        let fname_repo_rel = diff_paths(fname.as_path(), git_base_path.as_path()).unwrap();
        // println!("rel file to commit: {}", fname_repo_rel.display());
        index.add_path(fname_repo_rel.as_path()).unwrap();
    });
    // the modified in-memory index need to flush back to disk
    index.write().unwrap();

    // write the whole tree from the index to the repo object store
    // returns the object id you can use to lookup the actual tree object
    let new_tree_oid = index.write_tree().unwrap();
    // this is our new tree, i.e. the root directory of the new commit
    let new_tree = repo.find_tree(new_tree_oid).unwrap();

    // either use the configured author signature
    let author = repo.signature().unwrap();
    // or use an alternative signature. commiter and author need not be the same
    /* let author = Signature::now("nick", "nick@example.com"); */

    // for simple commit, use current head as parent
    // you need more than one parent if the commit is a merge
    let head = repo.head().unwrap();
    let parent = repo.find_commit(head.target().unwrap()).unwrap();
    let message = match &cli.change_type {
        VersionChangeType::Fixed { .. } => format!("Changed version in tomls to fixed version '{}'", new_version.to_string()),
        VersionChangeType::Increment { vtype } => format!("Changed version in tomls to '{}' by incrementing {}", new_version.to_string(), vtype),
        VersionChangeType::OnlyShow => { print_error("Commit called for 'OnlyShow' -> aborting".into()) }
    };
    let oid = repo.commit(Some("HEAD"), &author, &author, message.as_str(),  &new_tree, &[&parent], )
        .unwrap();
    println!("{INDENT}Cargo.tomls with updated version comitted (id: {})", oid);

    println!("       {} {} done", CHECK, txt);

    // ***
    let txt = String::from("Add git tag for version");
    println!("[4/5] {} {} ...", TAG, txt);

    let obj = repo.revparse_single("HEAD").unwrap();
    let r = repo.tag(git_tag_new_version_str.as_str(), &obj, &author, cli.tag_message.as_str(), false);
    if let Err(e) = r {
        print_error(format!("Error adding git tag {}: {}", git_tag_new_version_str, e));
    }
    println!("       {} {} done", CHECK, txt);

    // ***
    let txt = String::from("git push for cargo.toml(s) and tag");
    println!("[5/5] {} {} ...", TRUCK, txt);

    let branch_ref = repo.head().unwrap();
    let branch_ref_name = branch_ref.name().unwrap();
    //base_repo.set_head(branch_ref_name).unwrap();
    let tag_ref = format!("refs/tags/{}", git_tag_new_version_str);
    println!("{INDENT}pushing to remote '{}' with branch_ref_name '{}' and '{}'", git_remote.name().unwrap(), branch_ref_name, tag_ref);
    if let Err(e) = git_remote.push(&[branch_ref_name, tag_ref.as_str()], Some(&mut po)) {
        print_error(format!("Error pushing to git remote: {}", e));
    }

    println!("       {} {} done", CHECK, txt);

}
