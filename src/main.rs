mod lib;

use anyhow::{anyhow, Result};
use std::env;

use lib::generate;

const USAGE: &str = r#"
  Generates random passwords.

  Options:
    -c charset_spec     use this character set
    -l password_len     generate passwords of this length (default 24)
    -n num_password     generate this many passwords (default 1)

  The charset specification language is a subset of the character set language
  for regular expressions. Only characters and ranges are allowed. Literal
  hyphens and backslashes must be escaped. Other characters must not be
  escaped. An initial caret may be used to invert the character set with
  respect to typeable ASCII characters.
"#;

const DEFAULT_PASSWORD_LEN: usize = 24;
const DEFAULT_NUM_PASSWORDS: usize = 1;

fn main() {
    if let Err(msg) = run() {
        eprintln!("{}", msg);
    }
}

fn run() -> Result<()> {
    let args = env::args().collect::<Vec<_>>();
    let mut args = args.iter();
    args.next();
    let mut charset_spec = None;
    let mut password_len = DEFAULT_PASSWORD_LEN;
    let mut num_passwords = DEFAULT_NUM_PASSWORDS;
    loop {
        match (args.next().map(|s| s.as_str()), args.next()) {
            (Some("-c"), Some(charset_spec_value)) => {
                charset_spec = Some(charset_spec_value);
            },
            (Some("-l"), Some(password_len_str)) => {
                password_len = password_len_str.parse::<usize>()?;
                if password_len == 0 {
                    let msg = "password length must not be zero";
                    return Err(anyhow!(msg));
                }
            },
            (Some("-n"), Some(num_passwords_str)) => {
                num_passwords = num_passwords_str.parse::<usize>()?;
                if num_passwords == 0 {
                    let msg = "number of passwords must not be zero";
                    return Err(anyhow!(msg));
                }
            },
            (Some(_), _) => {
                return Err(anyhow!(usage()));
            },
            (None, _) => {
                break;
            },
        }
    }
    let passwords = generate(charset_spec, password_len, num_passwords)?;
    for password in passwords {
        println!("{}", password);
    }
    Ok(())
}

fn usage() -> String {
    let program_name = env::args().next().unwrap_or("".to_string());
    format!("\n  Usage: {} [options]\n{}", program_name, USAGE)
}
