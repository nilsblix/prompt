use std::{
    env,
    error::Error,
    fmt::{ self, Write },
};

#[allow(unused)]
#[derive(Debug)]
enum Color {
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
}

impl Color {
    fn to_ansi(&self) -> i32 {
        match self {
            Color::Red => 31,
            Color::Green => 32,
            Color::Yellow => 33,
            Color::Blue => 34,
            Color::Magenta => 35,
            Color::Cyan => 36,
            Color::White => 37,
        }
    }
}

#[derive(Debug)]
enum DecoratedString {
    Bold(Box<DecoratedString>),
    Colored(Box<DecoratedString>, Color),
    Default(String),
}

impl DecoratedString {
    fn append_to_ansi(val: &DecoratedString, s: &mut String, escape_fn: &dyn Fn(&str) -> String) -> Result<(), fmt::Error> {
        // https://gist.github.com/fnky/458719343aabd01cfb17a3a4f7296797
        match val {
            DecoratedString::Bold(inner) => {
                write!(s, "{}", escape_fn("\x1b[1m"))?;
                Self::append_to_ansi(inner, s, escape_fn)?;
                write!(s, "{}", escape_fn("\x1b[22m"))?;
            }
            DecoratedString::Colored(inner, color) => {
                write!(s, "{}", escape_fn(&format!("\x1b[{}m", color.to_ansi())))?;
                Self::append_to_ansi(inner, s, escape_fn)?;
                write!(s, "{}", escape_fn("\x1b[39m"))?;
            }
            DecoratedString::Default(val) => {
                write!(s, "{val}")?;
            }
        }

        Ok(())
    }

    fn to_ansi(&self, escape_fn: &dyn Fn(&str) -> String) -> String {
        let mut ret = String::new();
        Self::append_to_ansi(self, &mut ret, escape_fn).unwrap();
        ret
    }

    fn bold(self) -> DecoratedString {
        DecoratedString::Bold(Box::new(self))
    }

    fn colored(self, color: Color) -> DecoratedString {
        DecoratedString::Colored(Box::new(self), color)
    }

    fn new(s: String) -> DecoratedString {
        DecoratedString::Default(s)
    }
}

fn get_cwd() -> DecoratedString {
    let cwd = env::var("PWD");

    if cwd.is_err() {
        return DecoratedString::new("!!!".to_string())
            .bold()
            .colored(Color::Red);
    }

    let mut cwd = cwd.unwrap();

    if let Ok(home) = env::var("HOME") {
        if cwd.starts_with(&home) {
            cwd = cwd.replace(&home, "~");
        }
    }

    DecoratedString::new(cwd)
        .bold()
        .colored(Color::White)
}

#[derive(Debug)]
struct NotInNixShell;

impl Error for NotInNixShell {}

impl fmt::Display for NotInNixShell {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "not in a nix shell")
   }
}

// https://github.com/starship/starship/blob/master/src/modules/nix_shell.rs
enum NixShellType {
    Pure,
    Impure,
    /// We're in a Nix shell, but we don't know which type.
    /// This can only happen in a `nix shell` shell (not a `nix-shell` one).
    Unknown,
}

impl NixShellType {
    fn detect_shell_type() -> Result<Self, NotInNixShell> {
        use NixShellType::{Impure, Pure, Unknown};

        let shell_type = env::var("IN_NIX_SHELL");
        match shell_type {
            Ok(val) if val == "pure" => return Ok(Pure),
            Ok(val) if val == "impure" => return Ok(Impure),
            Ok(_) => return Ok(Unknown),
            _ => {},
        }

        // Hack to detect if we're in a `nix shell`
        let path = env::var("PATH").map_err(|_| NotInNixShell)?;
        let in_nix_shell = env::split_paths(&path)
            .any(|p: std::path::PathBuf| p.starts_with("/nix/store"));

        if in_nix_shell {
            Ok(Unknown)
        } else {
            Err(NotInNixShell)
        }
    }
}

fn get_nix_shell() -> Result<DecoratedString, NotInNixShell> {
    use NixShellType::{Impure, Pure, Unknown};

    let shell_type = NixShellType::detect_shell_type()?;

    let name = match shell_type {
        Pure => "pure",
        Impure => "impure",
        Unknown => "unknown",
    };

    Ok(DecoratedString::new(format!("(nix: {})", name))
        .bold()
        .colored(Color::Red))
}

#[derive(Debug)]
enum GitError {
    NoCwd(std::io::Error),
    CanonicalCwd(std::io::Error),
    ReadGitFile(std::io::Error),
    ReadHead(std::io::Error),
    NotGitRepo,
    UnexpectedGitContent,
    ReadRef(std::io::Error),
    NoRefName,
}

impl fmt::Display for GitError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GitError::NoCwd(_) => write!(f, "failed to get cwd"),
            GitError::CanonicalCwd(_) => write!(f, "failed to canonicalize cwd"),
            GitError::ReadGitFile(_) => write!(f, "failed to read .git file"),
            GitError::ReadHead(_) => write!(f, "failed to read git HEAD"),
            GitError::NotGitRepo => write!(f, "not a git repo"),
            GitError::UnexpectedGitContent => write!(f, "unexpected git content"),
            GitError::ReadRef(_) => write!(f, "failed to read ref"),
            GitError::NoRefName => write!(f, "failed to get ref name"),
        }
    }
}

impl Error for GitError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            GitError::NoCwd(e) => Some(e),
            GitError::CanonicalCwd(e) => Some(e),
            GitError::ReadGitFile(e) => Some(e),
            GitError::ReadHead(e) => Some(e),
            GitError::NotGitRepo => None,
            GitError::UnexpectedGitContent => None,
            GitError::ReadRef(e) => Some(e),
            GitError::NoRefName => None,
        }
    }
}

fn get_git_info() -> Result<DecoratedString, GitError> {
    use std::{
        fs,
        path::*,
    };

    let cwd = env::current_dir().map_err(GitError::NoCwd)?;
    let canonical_cwd = fs::canonicalize(cwd).map_err(GitError::CanonicalCwd)?;

    let mut dir_iter = Some(&canonical_cwd as &Path);
    while let Some(dir) = dir_iter {
        if dir.join(".git").exists() {
            break;
        }

        dir_iter = dir.parent();
    }

    let repo = dir_iter.ok_or(GitError::NotGitRepo)?;

    // if .git has gitdir:.... we have to follow the link

    let mut git_dir = repo.join(".git");
    if git_dir.is_file() {
        let git_content = fs::read_to_string(git_dir).map_err(GitError::ReadGitFile)?;

        const PREFIX: &str = "gitdir: ";

        match git_content.strip_prefix(PREFIX) {
            Some(v) => git_dir = v.trim().into(),
            None => return Err(GitError::UnexpectedGitContent),
        }
    }

    let head_content = fs::read_to_string(git_dir.join("HEAD")).map_err(GitError::ReadHead)?;

    const REF_PREFIX: &str = "ref: ";
    let output = match head_content.strip_prefix(REF_PREFIX) {
        Some(refs_path) => {
            let refs_path = Path::new(refs_path.trim());

            let commit_hash =
                fs::read_to_string(git_dir.join(refs_path)).map_err(GitError::ReadRef)?;

            let short_hash = &commit_hash[..5];
            let ref_name = refs_path
                .file_name()
                .ok_or(GitError::NoRefName)?
                .to_string_lossy();

            let extension = if commit_hash.chars().count() > 5 {
                ".."
            } else {
                ""
            };

            format!("{ref_name} {short_hash}{extension}")
        }
        None => head_content[..14].to_string(),
    };

    Ok(DecoratedString::new(output)
        .colored(Color::Red)
        .bold())
}

#[derive(Debug)]
enum MainError {
    NixShell(NotInNixShell),
    Git(GitError),
}

impl fmt::Display for MainError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let source: &dyn std::error::Error = match self {
            MainError::NixShell(e) => {
                writeln!(f, "failed to get nix info")?;
                e
            },
            MainError::Git(e) => {
                writeln!(f, "failed to get git info")?;
                e
            },
        };

        writeln!(f, "Caused by:")?;

        let mut source = Some(source);
        while let Some(err) = source {
            writeln!(f, "{err}")?;
            source = err.source();
        }

        Ok(())
    }
}

fn main() {
    // This program uses these environment variables:
    //
    // 1. `PROMPT_DEBUG`:
    //      1 => Print out debug stats
    //      0 => No debug
    // 2. `PROMPT_SHELL_TYPE`:
    //      'bash' => The current shell is bash
    //      'zsh' => The current shell is zsh
    //
    // Here is how to setup the prompt for zsh:
    // ```.zshrc
    // PROMPT="$(PROMPT_SHELL_TYPE='zsh' ./path/to/prompt/binary)"
    // ```

    let shell_type = env::var("PROMPT_SHELL_TYPE").expect("Prompt shell type is unspecified");

    let escape_ansi = |s: &str| -> String {
        if shell_type == "zsh" {
            format!("%{{{s}%}}")
        } else if shell_type == "bash" {
            format!("\\[{s}\\]")
        } else {
            s.to_string()
        }
    };

    let (oks, errors): (Vec<Result<_, MainError>>, Vec<_>) = vec![
        Ok(get_cwd()),
        get_git_info().map_err(MainError::Git),
        get_nix_shell().map_err(MainError::NixShell),
    ]
    .into_iter()
    .partition(Result::is_ok);

    let components: Vec<_> = oks
        .into_iter()
        .map(|x| x.expect("Invalid Result"))
        .collect();

    if Ok("1") == env::var("PROMPT_DEBUG").as_ref().map(|s| s.as_str()) {
        for error in errors.into_iter().map(|e| e.unwrap_err()) {
            eprintln!("{error}");
        }
    }

    let joined = components
        .into_iter()
        .map(|s| format!("{} ", s.to_ansi(&escape_ansi)))
        .collect::<Vec<_>>()
        .join("-");

    print!(" {joined}-> ");
}
