use anyhow::*;
use bitvec::array::BitArray;
use bitvec::prelude::Lsb0;
use num_bigint::BigUint;
use num_integer::Integer;
use num_traits::cast::ToPrimitive;
use rand::RngCore;
use rand::rngs::OsRng;

use self::CharsetParserState::*;

#[derive(Copy, Clone)]
enum CharsetParserState {
    Start,
    Char(u8),
    Escape,
    Range(u8),
    RangeEscape(u8),
}

const HYPHEN: u8 = 45;
const BACKSLASH: u8 = 92;
const CARET: u8 = 94;

const TYPEABLE: [u64; 2] = [0xffff_ffff_0000_0000, 0x7fff_ffff_ffff_ffff];

pub fn generate(
    charset_spec: Option<&String>,
    password_len: usize,
    num_passwords: usize,
) -> Result<Vec<String>> {
    let charset =
        match charset_spec {
            Some(charset_spec) =>
                parse_charset_spec(charset_spec)?,
            None => {
                let charset = BitArray::<_, Lsb0>::from(TYPEABLE);
                charset.iter_ones().map(|i| i as u8).collect()
            },
        };
    let base = charset.len();
    let mut value = {
        let total_chars = password_len * num_passwords;
        let num_bits = BigUint::from(base).pow(total_chars as u32).bits();
        let num_bytes = (num_bits / 8) + 1;
        let mut buffer = vec![0u8; num_bytes as usize];
        OsRng.fill_bytes(&mut buffer);
        BigUint::from_bytes_le(&buffer)
    };
    let base = base.into();
    let mut passwords = Vec::with_capacity(num_passwords);
    loop {
        let mut password_bytes = Vec::with_capacity(password_len);
        for _ in 0..password_len {
            let (quo, rem) = value.div_mod_floor(&base);
            value = quo;
            let idx = rem.to_usize().unwrap();
            let ch = charset[idx];
            password_bytes.push(ch);
        }
        let string = String::from_utf8(password_bytes).unwrap();
        passwords.push(string);
        if passwords.len() == num_passwords {
            break;
        }
    }
    Ok(passwords)
}

pub fn parse_charset_spec(charset_spec: &String) -> Result<Vec<u8>> {
    fn err_escape_hyphen() -> Result<Vec<u8>> {
        Err(anyhow!("hyphens must be escaped"))
    }
    fn err_invalid_escape(byte: u8) -> Result<Vec<u8>> {
        Err(anyhow!("invalid escape sequence: \"\\{}\"", byte as char))
    }
    if charset_spec.is_empty() {
        return Err(anyhow!("empty charset specification"));
    }
    let bytes = charset_spec.as_bytes();
    let invert = bytes[0] == CARET;
    let mut bytes = bytes.iter();
    if invert {
        bytes.next();
    }
    let mut state = Start;
    let mut result = BitArray::<_, Lsb0>::from([0u64; 2]);
    let typeable = BitArray::<_, Lsb0>::from(TYPEABLE);
    for &byte in bytes {
        if !typeable.get(byte as usize).unwrap() {
            return Err(anyhow!("found untypeable or non-ASCII character"));
        }
        match (state, byte) {
            (Start, HYPHEN) => {
                return err_escape_hyphen();
            },
            (Start, BACKSLASH) => {
                state = Escape;
            },
            (Start, byte) => {
                result.set(byte as usize, true);
                state = Char(byte);
            },
            (Char(prev), HYPHEN) => {
                state = Range(prev);
            },
            (Char(_), BACKSLASH) => {
                state = Escape;
            },
            (Char(_), byte) => {
                result.set(byte as usize, true);
            },
            (Escape, byte) => {
                if byte != HYPHEN && byte != BACKSLASH {
                    return err_invalid_escape(byte);
                }
                result.set(byte as usize, true);
                state = Char(byte);
            },
            (Range(_), HYPHEN) => {
                return err_escape_hyphen();
            },
            (Range(start), BACKSLASH) => {
                state = RangeEscape(start);
            },
            (Range(start), end) => {
                for byte in (start + 1)..=end {
                    result.set(byte as usize, true);
                }
                state = Start;
            },
            (RangeEscape(start), end) => {
                if byte != HYPHEN && byte != BACKSLASH {
                    return err_invalid_escape(byte);
                }
                for byte in (start + 1)..=end {
                    result.set(byte as usize, true);
                }
                state = Start;
            }
        }
    }
    match state {
        Escape | RangeEscape(_) =>
            Err(anyhow!("unterminated escape sequence")),
        Range(_) =>
            Err(anyhow!("unterminated character range")),
        _ => {
            if invert {
                let tmp = result;
                result = typeable;
                result &= !tmp;
            }
            if result.not_any() {
                return Err(anyhow!("character set is empty"))
            }
            Ok(result.iter_ones().map(|i| i as u8).collect())
        }
    }
}
