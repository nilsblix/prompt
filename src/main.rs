use std::{
    env, 
    error::Error, 
    fmt::{ self },
};

use colored::*;

#[derive(Debug)]
enum UserError {
    NoUser(std::io::Error),
}

impl Error for UserError {}

impl fmt::Display for UserError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            UserError::NoUser(e) => write!(f, "failed to get user: {}", e), 
        }
    }
}

fn get_user() -> Result<String, UserError> {
    let user = whoami::fallible::username().map_err(UserError::NoUser)?;
    Ok(user.magenta().bold().to_string())
}

#[derive(Debug)]
enum HostnameError {
    NoHost(std::io::Error),
}

impl Error for HostnameError {}

impl fmt::Display for HostnameError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            HostnameError::NoHost(e) => write!(f, "failed to get host: {}", e), 
        }
    }
}

fn get_hostname() -> Result<String, HostnameError> {
    let host = whoami::fallible::hostname().map_err(HostnameError::NoHost)?;
    Ok(host.bold().green().to_string())
}

fn get_cwd() -> String  {
    let cwd = env::var("PWD");

    if cwd.is_err() {
        return "!!!".bold().red().to_string();
    }

    let mut cwd = cwd.unwrap();
    
    if let Ok(home) = env::var("HOME") {
        if cwd.starts_with(&home) {
            cwd = cwd.replace(&home, "~");
        }
    }

    cwd.bold().blue().to_string()
}

#[derive(Debug)]
struct NotInNixShell;

impl Error for NotInNixShell {}

impl fmt::Display for NotInNixShell { fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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

fn get_nix_shell() -> Result<String, NotInNixShell> {
    use NixShellType::{Impure, Pure, Unknown};

    let shell_type = NixShellType::detect_shell_type()?;

    let name = match shell_type {
        Pure => "pure",
        Impure => "impure",
        Unknown => "unknown",
    };

    Ok(format!("nix: {}", name).white().bold().to_string())
}

#[derive(Debug)]
enum MainError {
    User(UserError),
    Hostname(HostnameError),
    NixShell(NotInNixShell),
}

impl fmt::Display for MainError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let source: &dyn std::error::Error = match self {
            MainError::User(e) => {
                writeln!(f, "failed to get user info")?;
                e
            },
            MainError::Hostname(e) => {
                writeln!(f, "failed to get hostname info")?;
                e
            },
            MainError::NixShell(e) => {
                writeln!(f, "failed to get nix info")?;
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

fn do_print(mut components: Vec<String>) {
    components.insert(0, "[".into());
    for i in 1..components.len() - 1 {
        components.insert(2 * i, "]-[".into());
    }
    components.push("] -> ".into());
    for component in components {
        print!("{component}");
    }
}

fn main() {
    colored::control::set_override(true);

    let (oks, errors): (Vec<Result<_, MainError>>, Vec<_>) = vec![
        get_user().map_err(MainError::User),
        get_hostname().map_err(MainError::Hostname),
        Ok(get_cwd()),
        get_nix_shell().map_err(MainError::NixShell),
    ]
    .into_iter()
    .partition(Result::is_ok);

    let components: Vec<_> = oks
        .into_iter()
        .map(|x| x.expect("Invalid Result"))
        .collect();

    if Ok("1") == env::var("DEBUG_PROMPT").as_ref().map(|s| s.as_str()) {
        for error in errors.into_iter().map(|e| e.unwrap_err()) {
            eprintln!("{error}");
        }
    }

    do_print(components);
}
