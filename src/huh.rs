//! The huH codec: bytes as base-3 text over the alphabet h / u / H, chunks joined with U.
//!
//! Three bytes become one number (offset by 65792 so it can't be mistaken for a
//! shorter chunk), a trailing pair becomes one offset by 256, and a trailing single
//! byte is its own value. Each number is written in base 3, most significant digit
//! first, with no leading zeros, so 0 is the empty string.

const DIGITS: [u8; 3] = [b'h', b'u', b'H'];
const SEP: u8 = b'U';

const THREE: u32 = 65_792;
const TWO: u32 = 256;
/// The largest number a three-byte chunk can make.
const MAX: u64 = 0xff_ffff + THREE as u64;

/// True for text made only of h, u, H and U (and at least one of them).
pub fn is_huh(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| matches!(b, b'h' | b'u' | b'H' | b'U'))
}

/// `String.prototype.trim` as the browser does it: Unicode white space plus the BOM.
pub fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}')
}

/// What the page treats as "this is a huH": long enough, and nothing but huH letters.
pub fn looks_like_huh(s: &str) -> bool {
    let t = js_trim(s);
    t.len() > 8 && is_huh(t)
}

fn push_base3(out: &mut Vec<u8>, mut n: u32) {
    let mut buf = [0u8; 21];
    let mut at = buf.len();
    while n > 0 {
        at -= 1;
        buf[at] = DIGITS[(n % 3) as usize];
        n /= 3;
    }
    out.extend_from_slice(&buf[at..]);
}

/// Bytes to huH.
pub fn encode(bytes: &[u8]) -> String {
    let mut out = Vec::with_capacity(bytes.len() / 3 * 17 + 40);
    for (i, c) in bytes.chunks(3).enumerate() {
        if i > 0 {
            out.push(SEP);
        }
        let n = match *c {
            [a, b, c] => (((a as u32) << 16) | ((b as u32) << 8) | c as u32) + THREE,
            [a, b] => (((a as u32) << 8) | b as u32) + TWO,
            [a] => a as u32,
            _ => unreachable!(),
        };
        push_base3(&mut out, n);
    }
    // Only ASCII letters were pushed.
    String::from_utf8(out).expect("huH is ASCII")
}

/// The text held a character that isn't h, u, H or U.
#[derive(Debug, PartialEq, Eq)]
pub struct NotHuh;

/// huH back to the bytes it was made from.
///
/// Two formats are read. Today's (the first number is above 255) packs bytes as
/// `encode` does. The first-ever format wrote every character of the base64
/// ciphertext as its own number; that is turned back into base64 and decoded.
/// Well-formed huH letters that spell nonsense come back as no bytes at all, which
/// then fails to decrypt like any other wrong input.
pub fn decode(msg: &str) -> Result<Vec<u8>, NotHuh> {
    let mut numbers = Numbers { bytes: msg.as_bytes(), at: 0, done: false };
    let first = match numbers.next() {
        Some(n) => n?,
        None => return Ok(Vec::new()),
    };

    if first <= 255 {
        // Legacy: one number per base64 character.
        let mut b64 = String::with_capacity(msg.len() / 4);
        for n in std::iter::once(Ok(first)).chain(numbers) {
            match char::from_u32(n?.min(u32::MAX as u64) as u32) {
                Some(c) if c.is_ascii() => b64.push(c),
                _ => return Ok(Vec::new()),
            }
        }
        return Ok(crate::b64::decode(&b64).unwrap_or_default());
    }

    let mut out = Vec::with_capacity(msg.len() / 5);
    for n in std::iter::once(Ok(first)).chain(numbers) {
        let n = n?;
        if n > MAX {
            return Ok(Vec::new());
        }
        let n = n as u32;
        if n >= THREE {
            let v = n - THREE;
            out.extend_from_slice(&[(v >> 16) as u8, (v >> 8) as u8, v as u8]);
        } else if n >= TWO {
            let v = n - TWO;
            out.extend_from_slice(&[(v >> 8) as u8, v as u8]);
        } else {
            out.push(n as u8);
        }
    }
    Ok(out)
}

/// The U-separated base-3 numbers of a huH, one at a time.
struct Numbers<'a> {
    bytes: &'a [u8],
    at: usize,
    done: bool,
}

impl Iterator for Numbers<'_> {
    type Item = Result<u64, NotHuh>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        let mut n: u64 = 0;
        loop {
            let Some(&b) = self.bytes.get(self.at) else {
                self.done = true;
                return Some(Ok(n));
            };
            self.at += 1;
            let digit = match b {
                b'h' => 0,
                b'u' => 1,
                b'H' => 2,
                SEP => return Some(Ok(n)),
                _ => {
                    self.done = true;
                    return Some(Err(NotHuh));
                }
            };
            // Saturate: anything this large is nonsense either way.
            n = n.saturating_mul(3).saturating_add(digit);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_is_empty() {
        assert_eq!(encode(&[0]), "");
        assert_eq!(encode(&[1]), "u");
        assert_eq!(encode(&[2]), "H");
        assert_eq!(encode(&[3]), "uh");
    }

    #[test]
    fn round_trips_every_tail_length() {
        for len in 0..40usize {
            // Start high so the first chunk is in today's format (above 255).
            let bytes: Vec<u8> = (0..len).map(|i| (200 + i * 37) as u8).collect();
            let huh = encode(&bytes);
            if len >= 2 {
                assert_eq!(decode(&huh).unwrap(), bytes, "len {len}");
            }
        }
    }

    #[test]
    fn rejects_other_letters() {
        assert_eq!(decode("huHUx"), Err(NotHuh));
        assert!(!is_huh(""));
        assert!(!is_huh("hello"));
        assert!(is_huh("huHU"));
    }

    #[test]
    fn nonsense_numbers_decode_to_nothing() {
        let huge = "H".repeat(40);
        assert_eq!(decode(&format!("{huge}U{huge}")).unwrap(), Vec::<u8>::new());
    }

    #[test]
    fn trims_like_the_browser() {
        assert_eq!(js_trim("\u{feff} huH \n"), "huH");
        assert!(looks_like_huh("  huHUhuHUh\n"));
        assert!(!looks_like_huh("huHUhuHU"));
    }
}
