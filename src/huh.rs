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

/// The first byte a huH in today's format decodes to, and how many bytes it
/// decodes to in all, worked out from its first and last numbers and a count of
/// the ones between (every number but the last is three bytes). Enough to tell
/// what kind of thing a huH is before the work of reading a long one. `None`
/// for the first huH format, and for anything that isn't a huH.
pub fn peek(msg: &str) -> Option<(u8, usize)> {
    let number = |chunk: &str| Numbers { bytes: chunk.as_bytes(), at: 0, done: false }.next()?.ok();
    let first = number(msg.split(SEP as char).next()?)?;
    let first_byte = match first {
        // Today's format always starts above 255 (see `decode`).
        n if n <= 255 || n > MAX => return None,
        n if n >= u64::from(THREE) => ((n - u64::from(THREE)) >> 16) as u8,
        n => ((n - u64::from(TWO)) >> 8) as u8,
    };
    let last_size = match number(msg.rsplit(SEP as char).next()?)? {
        n if n > MAX => return None,
        n if n >= u64::from(THREE) => 3,
        n if n >= u64::from(TWO) => 2,
        _ => 1,
    };
    let before_last = msg.bytes().filter(|&b| b == SEP).count();
    Some((first_byte, 3 * before_last + last_size))
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
    fn peeks_at_the_first_byte_and_the_length() {
        for len in 2..60usize {
            for start in [0u8, 1, 2, 0x53, 200] {
                let bytes: Vec<u8> = (0..len).map(|i| start.wrapping_add((i * 37) as u8)).collect();
                assert_eq!(peek(&encode(&bytes)), Some((start, len)), "{len} bytes from {start}");
                // A tail of zeros is short to write (nothing at all, for one zero) and still counted.
                let zeros: Vec<u8> = [&[start, 9, 9][..], &vec![0u8; len]].concat();
                assert_eq!(peek(&encode(&zeros)), Some((start, len + 3)), "{len} zeros after {start}");
            }
        }
        // The first huH format (a number for every base64 letter), one byte alone, and what isn't huH.
        for not_todays in ["uhhuuUuHuHUHuHu", "u", "", "huHUx", "hello"] {
            assert_eq!(peek(not_todays), None, "{not_todays:?}");
        }
        let huge = "H".repeat(40);
        assert_eq!(peek(&huge), None);
        assert_eq!(peek(&format!("{}U{huge}", encode(&[9, 9, 9]))), None);
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
