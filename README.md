# Cargo Git Version Setter (cgvs)

A command-line tool for updating versions in one or more `Cargo.toml` files, creating a Git tag, and committing the changes.

## Features
- Update the version number in one or multiple `Cargo.toml` files.
- Create a Git tag for the new version.
- Commit the changes automatically.
- Supports version increments and fixed version setting.
- Can scan subdirectories for `Cargo.toml` files.
- Pushes changes and tags to a remote repository.

## Installation
Ensure you have Rust and Cargo installed on your system. Then, install the tool using:

```sh
cargo install cargo-git-version-setter
```

Alternatively, you can build from source:

```sh
git clone https://github.com/yourusername/cargo-git-version-setter.git
cd cargo-git-version-setter
cargo build --release
```

## Usage
Run the tool with a command:

```sh
cgvs [OPTIONS] <COMMAND>
```

### Commands
- `fixed <FULL_VERSION>` - Set a fixed version.
- `increment <VTYPE>` - Increment part of the version. When incrementing major or minor version parts, the lower version parts are set to zero.
- `only-show` - Show versions from Cargo and Git, then exit.

### Command Details
#### Set Fixed Version
Set a specific version for the project.

**Usage:**
```sh
cgvs fixed <FULL_VERSION>
```

**Arguments:**
- `<FULL_VERSION>` - The version to set.

**Options:**
- `-h, --help` - Print help information.

#### Increment Version Part
Increment a specific part of the version. When incrementing major or minor version parts, the lower version parts are reset to zero.

**Usage:**
```sh
cgvs increment <VTYPE>
```

**Arguments:**
- `<VTYPE>` - The version part to increment. Possible values:
    - `patch` - Patch version for backward-compatible bug fixes.
    - `minor` - Minor version for backward-compatible feature additions.
    - `major` - Major version for breaking API changes.

**Options:**
- `-h, --help` - Print help information.

#### Only Show Versions
Display versions from Cargo and Git, then exit without making changes.

**Usage:**
```sh
cgvs only-show
```

**Options:**
- `-h, --help` - Print help information.

### Options
- `-p, --path <PATH>` - Path of the project.
- `-c, --cargo-file-selector <CARGO_FILE_SELECTOR>` - Select cargo file if multiple exist (`leaf`, `base`, or `all`).
- `-s, --scan-subdirs` - Scan subdirectories for `Cargo.toml` files.
- `-v, --verbose` - Enable debugging output.
- `-t, --tag-message <TAG_MESSAGE>` - Message when adding the tag to Git.
- `-r, --remote <REMOTE>` - Git remote name to push new commits to (default: `origin`).
- `-g, --git-prefix-for-tag <GIT_PREFIX_FOR_TAG>` - Prefix for the version tag (default: `v`).
- `-h, --help` - Show help.
- `-V, --version` - Show version.

## Workflow
1. Updates the `version` field in all detected `Cargo.toml` files.
2. Stages and commits the changes with a message (default: `chore: bump version to <new-version>`).
3. Creates a Git tag for the new version.
4. Pushes the commit and tag (if `--do-push` is used).

## Examples
To set a fixed version and create a Git tag:

```sh
cgvs fixed 2.0.0
```

To increment the minor version:

```sh
cgvs increment minor
```

To show current versions without making changes:

```sh
cgvs only-show
```

To scan subdirectories and apply changes:

```sh
cgvs fixed 2.0.0 --scan-subdirs
```

## License
This project is licensed under the MIT License.

## Contributing
Feel free to submit issues or pull requests to improve this tool!

