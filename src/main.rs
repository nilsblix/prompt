use std::{
    env, 
    error::Error, 
    fmt::{ self, Write, },
};

#[allow(unused)]
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

enum DecoratedString {
    Bold(Box<DecoratedString>),
    Colored(Box<DecoratedString>, Color),
    Default(String),
}

impl DecoratedString {
    fn append_to_ansi(val: &DecoratedString, s: &mut String) -> Result<(), fmt::Error> {
        // https://gist.github.com/fnky/458719343aabd01cfb17a3a4f7296797
        match val {
            DecoratedString::Bold(inner) => {
                write!(s, "\x1b[1m")?;
                Self::append_to_ansi(inner, s)?;
                write!(s, "\x1b[22m")?;
            }
            DecoratedString::Colored(inner, color) => {
                write!(s, "\x1b[{}m", color.to_ansi())?;
                Self::append_to_ansi(inner, s)?;
                write!(s, "\x1b[39m")?;
            }
            DecoratedString::Default(val) => {
                write!(s, "{val}")?;
            }
        }

        Ok(())
    }

    fn to_ansi(&self) -> String {
        let mut ret = String::new();
        Self::append_to_ansi(self, &mut ret).unwrap();
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

fn get_time() -> String {
    let time = format!("{}", chrono::Local::now().time().format("%H:%M"));
    DecoratedString::new(time)
        .bold()
        .colored(Color::Cyan)
        .to_ansi()
}

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
    let dec = DecoratedString::new(user)
        .bold()
        .colored(Color::Magenta)
        .to_ansi();
    Ok(dec)
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
    let dec = DecoratedString::new(host)
        .bold()
        .colored(Color::Green)
        .to_ansi();
    Ok(dec)
}

fn get_cwd() -> String  {
    let cwd = env::var("PWD");

    if cwd.is_err() {
        return DecoratedString::new("!!!".to_string())
            .bold()
            .colored(Color::Red)
            .to_ansi();
    }

    let mut cwd = cwd.unwrap();
    
    if let Ok(home) = env::var("HOME") {
        if cwd.starts_with(&home) {
            cwd = cwd.replace(&home, "~");
        }
    }

    DecoratedString::new(cwd)
        .bold()
        .colored(Color::Blue)
        .to_ansi()
}

#[derive(Debug)]
enum ShellError {
    PsCommandFailed,
    PsCommandTerminationUnsuccessful,
    StdoutToStringFailed,
}

impl Error for ShellError {}

impl fmt::Display for ShellError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::PsCommandFailed => write!(f, "the command: `ps -p $$ -o comm=` has failed"),
            Self::PsCommandTerminationUnsuccessful => write!(f, "the command: `ps -p $$ -o comm=` was unsuccessful"),
            Self::StdoutToStringFailed => write!(f, "stdout to string failed"),
        }
    }
}

fn get_shell() -> Result<String, ShellError> {
    use std::process::Command;

    // This spawns a new shell, so to look for the current shell type we need to go two layers up:
    // 1 User shell session
    // 2 prompt session
    // 3 spawned sh to determine shell type of session 1
    let output = Command::new("sh")
        .arg("-c")
        .arg("ps -p $(ps -o ppid= -p $(ps -o ppid= -p $$)) -o comm=")
        .output()
        .map_err(|_| ShellError::PsCommandFailed)?;

    if !output.status.success() {
        return Err(ShellError::PsCommandTerminationUnsuccessful);
    }

    let string = String::from_utf8(output.stdout)
        .map_err(|_| ShellError::StdoutToStringFailed)?
        .trim()
        .to_string();

    Ok(DecoratedString::new(string)
        .bold()
        .colored(Color::White)
        .to_ansi())
}

#[derive(Debug)]
enum StatusError {
    NoExitCode,
    CodeNotANumber
}

impl Error for StatusError {}

impl fmt::Display for StatusError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::NoExitCode => write!(f, "no status/exit-code was found"),
            Self::CodeNotANumber => write!(f, "a non-number exit code was returned"),
        }
    }
}

fn get_exit_code() -> Result<String, StatusError> {
    let code = env::var("PROMPT_LAST_STATUS").map_err(|_| StatusError::NoExitCode)?;

    let val = code.parse::<i32>().map_err(|_| StatusError::CodeNotANumber)?;
    Ok(DecoratedString::new(code)
        .bold()
        .colored(match val {
            0 => Color::Green,
            _ => Color::Red,
        })
        .to_ansi())
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

fn get_nix_shell() -> Result<String, NotInNixShell> {
    use NixShellType::{Impure, Pure, Unknown};

    let shell_type = NixShellType::detect_shell_type()?;

    let name = match shell_type {
        Pure => "pure",
        Impure => "impure",
        Unknown => "unknown",
    };

    Ok(DecoratedString::new(format!("nix: {}", name))
        .bold()
        .to_ansi())
}

#[derive(Debug)]
enum MainError {
    User(UserError),
    Hostname(HostnameError),
    Shell(ShellError),
    Status(StatusError),
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
            MainError::Shell(e) => {
                writeln!(f, "failed to get shell info")?;
                e
            },
            MainError::Status(e) => {
                writeln!(f, "failed to get status")?;
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

// fn do_print(mut components: Vec<String>) {
//     components.insert(0, "┌[".into());
//     for i in 1..components.len() - 1 {
//         components.insert(2 * i, "]-[".into());
//     }
//     components.push("]\n└> ".into());
//     for component in components {
//         print!("{component}");
//     }
// }

fn do_print(mut components: Vec<String>) {
    components.insert(0, "[".into());
    for i in 1..components.len() - 1 {
        components.insert(2 * i, "]-[".into());
    }
    components.push("] -> ".into());
    for component in components {
        print!("{component}");
    }
    // Ensure all ANSI formatting is completely reset before the cursor position
    print!("\x1b[0m");
}

fn main() {
    // This program uses these environment variables:
    //
    // 1. `DEBUG_PROMPT`: 
    //      1 => Print out debug stats
    //      0 => No debug
    // 2. `PROMPT_LAST_STATUS`:
    //      Set to `$?` for an accurate last status.
    //      Doesn't have to be set if no exit status is wanted
    //
    // Here is how to setup the prompt for zsh:
    // ```.zshrc
    // PROMPT="$(PROMPT_LAST_STATUS=$? ./path/to/prompt/binary))"
    // ```

    let (oks, errors): (Vec<Result<_, MainError>>, Vec<_>) = vec![
        Ok(get_time()),
        get_user().map_err(MainError::User),
        get_hostname().map_err(MainError::Hostname),
        Ok(get_cwd()),
        get_shell().map_err(MainError::Shell),
        get_exit_code().map_err(MainError::Status),
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
