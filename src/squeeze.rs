//! Squeeze: LZMA2 compression of whatever is about to be encrypted.
//!
//! A squeezed payload starts with NUL (like a file payload, so typed text can't
//! start with it) and a letter saying how the rest is wrapped:
//!
//! * `L`, long: `length (LEB128) · raw LZMA2 stream · CRC-32 of the original`.
//!   Takes anything up to [`MAX`].
//! * `S` `T` `U` `V`, short: `length (LEB128) · range-coded bytes`. One LZMA2 chunk
//!   with everything that can be worked out again left off: the chunk header
//!   (rebuilt from the length, the byte count and the letter, which names one of
//!   the four [`MODELS`]), the zero every range-coded chunk begins with, and the
//!   end-of-stream byte. Up to 64 KiB.
//! * `W` `X` `Y` `Z`, words: the same as short, but the chunk was made with the
//!   word list in [`crate::words`] already in the window, so common English costs
//!   very little.
//!
//! Anything without one of those markers is exactly what it was before squeezing
//! existed, so older huHs still unlock, and a payload that won't shrink (a JPEG, a
//! two-word note) is stored the way it always was.
//!
//! Every form comes back whole or not at all. Long is checked against its length
//! and a checksum. The short forms carry no checksum, and inside the AES envelope
//! they don't need one: a compressed chunk only decodes if the range coder ends
//! exactly on its last byte with nothing left over, and AES-CBC never hands over
//! a payload with just a bit or two changed. Damage to a huH scrambles a whole
//! 16-byte block of what comes out (or the padding, or the marker), and the range
//! coder cannot ride that out. A lone flipped bit in the first few coded bytes
//! would slip through, because those are coded at even odds; it has no way to
//! arrive here on its own.

use std::io::{Read, Write};

use lzma_rust2::{EncodeMode, Lzma2Options, Lzma2Reader, Lzma2Writer, LzmaOptions, MfType};

use crate::{cipher, words::WORDS};

const NUL: u8 = 0x00;
const LONG: u8 = b'L';
/// The first of the four short letters, one per model.
const SHORT: u8 = b'S';
/// The first of the four short letters for chunks made with the word list.
const PRIMED: u8 = b'W';
const LAST: u8 = b'Z';

/// Nothing bigger than this is squeezed, so nothing bigger is ever inflated: a
/// forged huH can't balloon into gigabytes on unlock.
pub const MAX: usize = 40 * 1024 * 1024;

/// What fits in a single chunk with a two-byte length: the limit of the short forms.
const SHORT_MAX: usize = 1 << 16;

/// The window never goes above this. It is part of the format: unlock sizes its
/// window by the same rule.
const DICT_MAX: u32 = 8 << 20;
const DICT_MIN: u32 = 4096;

/// Up to here the slowest, most thorough match search costs next to nothing.
const THOROUGH_BELOW: usize = 1 << 20;

/// Big payloads are sampled first, so a 10 MB photo isn't ground through LZMA for nothing.
const PROBE_ABOVE: usize = 1 << 20;
const PROBE_SLICES: usize = 4;
const PROBE_SLICE: usize = 48 * 1024;

/// The header of a compressed LZMA2 chunk that also sets the properties:
/// control, length - 1 (16 bits), coded size - 1 (16 bits), properties.
const CHUNK_HEADER: usize = 6;
/// A range-coded chunk is never shorter than its leading zero and four bytes of state.
const CODED_MIN: usize = 5;
const CODED_MAX: usize = 1 << 16;

#[derive(Debug, PartialEq, Eq)]
pub enum SqueezeError {
    /// The payload starts with a marker by itself and is too big to wrap.
    Unwrappable,
}

/// The payload could not be put back: cut short, altered, or not ours.
#[derive(Debug, PartialEq, Eq)]
pub struct Damaged;

pub fn is_squeezed(payload: &[u8]) -> bool {
    matches!(payload, [NUL, LONG | SHORT..=LAST, ..])
}

fn window_for(len: usize) -> u32 {
    (len.next_power_of_two().min(DICT_MAX as usize) as u32).clamp(DICT_MIN, DICT_MAX)
}

/// How the literal coder is set up: (literal context bits, position bits).
/// LZMA's stock choice suits most things; plain text, and anything short, tends
/// to do a little better with less context to warm up.
type Model = (u32, u32);
const STOCK: Model = (LzmaOptions::LC_DEFAULT, LzmaOptions::PB_DEFAULT);

/// The models that get tried. A short form says which one it used by its letter
/// (first model, first letter), so this list and its order are part of the format.
const MODELS: [Model; 4] = [STOCK, (3, 0), (2, 0), (0, 0)];

/// The LZMA properties byte of a model (literal position bits are always 0).
fn props((lc, pb): Model) -> u8 {
    (pb * 5 * 9 + lc) as u8
}

/// What a short form's letter says: whether the word list was used, and the model.
fn short_kind(letter: u8) -> Option<(bool, Model)> {
    let (primed, at) = match letter {
        SHORT..PRIMED => (false, letter - SHORT),
        PRIMED..=LAST => (true, letter - PRIMED),
        _ => return None,
    };
    Some((primed, MODELS[at as usize]))
}

/// A raw LZMA2 stream of `raw`. With `preset`, the window starts out holding it.
fn encode(raw: &[u8], fast: bool, (lc, pb): Model, preset: Option<&[u8]>) -> Vec<u8> {
    let mut o = LzmaOptions::with_preset(if fast { 1 } else { 9 });
    o.dict_size = window_for(raw.len() + preset.map_or(0, <[u8]>::len));
    o.lc = lc;
    o.pb = pb;
    o.preset_dict = preset.map(<[u8]>::to_vec);
    if !fast {
        o.mode = EncodeMode::Normal;
        o.mf = MfType::Bt4;
        if raw.len() <= THOROUGH_BELOW {
            o.nice_len = LzmaOptions::NICE_LEN_MAX;
            o.depth_limit = 512;
        }
    }
    let mut w = Lzma2Writer::new(Vec::new(), Lzma2Options { lzma_options: o, chunk_size: None });
    w.write_all(raw).expect("writing to memory");
    w.finish().expect("writing to memory")
}

/// Decodes a raw LZMA2 stream that must hold exactly `len` bytes and then end.
/// Returns what it held and whatever follows the stream.
fn decode<'a>(
    stream: &'a [u8],
    len: usize,
    preset: Option<&[u8]>,
) -> Result<(Vec<u8>, &'a [u8]), Damaged> {
    let window = window_for(len + preset.map_or(0, <[u8]>::len));
    let mut reader = Lzma2Reader::new(stream, window, preset);
    let mut out = vec![0u8; len];
    reader.read_exact(&mut out).map_err(|_| Damaged)?;
    let mut extra = [0u8; 1];
    if reader.read(&mut extra).map_err(|_| Damaged)? != 0 {
        return Err(Damaged);
    }
    Ok((out, reader.into_inner()))
}

fn push_len(out: &mut Vec<u8>, mut n: usize) {
    loop {
        let b = (n & 0x7f) as u8;
        n >>= 7;
        if n == 0 {
            out.push(b);
            return;
        }
        out.push(b | 0x80);
    }
}

fn read_len(bytes: &[u8]) -> Option<(usize, usize)> {
    // Gathered in 64 bits so that a forged length reads the same (and is refused
    // the same) on the 32-bit WebAssembly build as anywhere else.
    let mut n: u64 = 0;
    for (i, &b) in bytes.iter().enumerate().take(5) {
        n |= u64::from(b & 0x7f) << (7 * i);
        if b & 0x80 == 0 {
            return usize::try_from(n).ok().map(|n| (n, i + 1));
        }
    }
    None
}

/// The long form of `raw`, around a stream already made of it.
fn pack_long(raw: &[u8], stream: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(stream.len() + 11);
    out.extend_from_slice(&[NUL, LONG]);
    push_len(&mut out, raw.len());
    out.extend_from_slice(stream);
    out.extend_from_slice(&crc32fast::hash(raw).to_le_bytes());
    out
}

fn open_long(len: usize, body: &[u8]) -> Result<Vec<u8>, Damaged> {
    if len > MAX {
        return Err(Damaged);
    }
    let (out, rest) = decode(body, len, None)?;
    if rest != crc32fast::hash(&out).to_le_bytes() {
        return Err(Damaged);
    }
    Ok(out)
}

/// The header LZMA2 puts on a stream's first chunk when that chunk is compressed.
/// `None` for sizes a single chunk of the short form can't have.
fn chunk_header(len: usize, coded: usize, primed: bool, model: Model) -> Option<[u8; CHUNK_HEADER]> {
    if !(1..=SHORT_MAX).contains(&len) || !(CODED_MIN..=CODED_MAX).contains(&coded) {
        return None;
    }
    // "New properties", and "start from an empty window" unless the word list is in it.
    let control = if primed { 0xC0 } else { 0xE0 };
    let (len, coded) = ((len - 1) as u16, (coded - 1) as u16);
    Some([control, (len >> 8) as u8, len as u8, (coded >> 8) as u8, coded as u8, props(model)])
}

/// The short form of a stream made with `MODELS[at]`, if the stream is exactly
/// one compressed chunk: `header · 0x00 · coded bytes · 0x00` becomes `coded bytes`.
fn pack_short(primed: bool, at: usize, len: usize, stream: &[u8]) -> Option<Vec<u8>> {
    if stream.len() < CHUNK_HEADER + CODED_MIN + 1 {
        return None;
    }
    let (header, rest) = stream.split_at(CHUNK_HEADER);
    let (coded, end) = rest.split_at(rest.len() - 1);
    // Only what `open_short` will rebuild byte for byte may be left off.
    if header != chunk_header(len, coded.len(), primed, MODELS[at])? || coded[0] != NUL || end != [NUL] {
        return None;
    }
    let mut out = Vec::with_capacity(coded.len() + 4);
    out.extend_from_slice(&[NUL, if primed { PRIMED } else { SHORT } + at as u8]);
    push_len(&mut out, len);
    out.extend_from_slice(&coded[1..]);
    Some(out)
}

fn open_short(len: usize, coded: &[u8], model: Model, preset: Option<&[u8]>) -> Result<Vec<u8>, Damaged> {
    let header = chunk_header(len, coded.len() + 1, preset.is_some(), model).ok_or(Damaged)?;
    let mut stream = Vec::with_capacity(CHUNK_HEADER + coded.len() + 2);
    stream.extend_from_slice(&header);
    stream.push(NUL);
    stream.extend_from_slice(coded);
    stream.push(NUL);
    match decode(&stream, len, preset)? {
        (out, []) => Ok(out),
        _ => Err(Damaged),
    }
}

/// True when evenly spaced samples of a big payload barely shrink at all.
fn looks_packed_already(raw: &[u8]) -> bool {
    let step = raw.len() / PROBE_SLICES;
    let (mut seen, mut made) = (0usize, 0usize);
    for i in 0..PROBE_SLICES {
        let slice = &raw[i * step..(i * step + PROBE_SLICE).min(raw.len())];
        seen += slice.len();
        made += encode(slice, true, STOCK, None).len();
    }
    made * 100 >= seen * 98
}

/// Wraps `raw` in whichever form comes out shortest.
///
/// Small payloads are cheap enough to squeeze every way (each model, with and
/// without the word list) and keep the best. The result says how it was made, so
/// unlock needs no hint.
fn wrap(raw: &[u8]) -> Vec<u8> {
    if raw.len() > SHORT_MAX {
        return pack_long(raw, &encode(raw, false, STOCK, None));
    }
    let mut best: Option<Vec<u8>> = None;
    let mut offer = |candidate: Vec<u8>| {
        if best.as_ref().is_none_or(|b| candidate.len() < b.len()) {
            best = Some(candidate);
        }
    };
    for (at, model) in MODELS.into_iter().enumerate() {
        let stream = encode(raw, false, model, None);
        // A stream that isn't one compressed chunk (nothing in it shrank) still fits the long form.
        offer(pack_short(false, at, raw.len(), &stream).unwrap_or_else(|| pack_long(raw, &stream)));
        // The word list still pays for itself at the top of this range, so it is always tried.
        let stream = encode(raw, false, model, Some(WORDS.as_bytes()));
        if let Some(short) = pack_short(true, at, raw.len(), &stream) {
            offer(short);
        }
    }
    best.expect("every model offers a candidate")
}

/// Returns the payload to encrypt: squeezed if that makes the huH shorter, else as given.
///
/// AES pads to whole 16-byte blocks, so only a whole block saved counts.
pub fn squeeze(raw: Vec<u8>) -> Result<Vec<u8>, SqueezeError> {
    // A payload that happens to open with a marker has to be wrapped, or unlock would misread it.
    let clash = is_squeezed(&raw);
    if raw.len() > MAX {
        return if clash { Err(SqueezeError::Unwrappable) } else { Ok(raw) };
    }
    if !clash {
        // Under one block there is nothing to save, and packed data isn't worth the grind.
        if cipher::blocks(raw.len()) == 1 || (raw.len() > PROBE_ABOVE && looks_packed_already(&raw)) {
            return Ok(raw);
        }
    }
    let packed = wrap(&raw);
    if !clash && cipher::blocks(packed.len()) >= cipher::blocks(raw.len()) {
        return Ok(raw);
    }
    Ok(packed)
}

/// The original payload of a squeezed one (check `is_squeezed` first).
pub fn unsqueeze(payload: &[u8]) -> Result<Vec<u8>, Damaged> {
    let [NUL, letter, body @ ..] = payload else { return Err(Damaged) };
    let (len, used) = read_len(body).ok_or(Damaged)?;
    let body = &body[used..];
    if *letter == LONG {
        return open_long(len, body);
    }
    let (primed, model) = short_kind(*letter).ok_or(Damaged)?;
    open_short(len, body, model, primed.then_some(WORDS.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn noise(n: usize) -> Vec<u8> {
        let mut v = vec![0u8; n];
        crate::rng::fill(&mut v);
        v
    }

    fn long_of(raw: &[u8]) -> Vec<u8> {
        pack_long(raw, &encode(raw, false, STOCK, None))
    }

    fn short_of(raw: &[u8]) -> Vec<u8> {
        pack_short(false, 0, raw.len(), &encode(raw, false, MODELS[0], None)).expect("one compressed chunk")
    }

    fn primed_of(raw: &[u8]) -> Vec<u8> {
        let stream = encode(raw, false, MODELS[3], Some(WORDS.as_bytes()));
        pack_short(true, 3, raw.len(), &stream).expect("one compressed chunk")
    }

    /// Squeezes, checks it comes back, and says which form was used (if any).
    fn there_and_back(raw: &[u8]) -> Option<u8> {
        let s = squeeze(raw.to_vec()).unwrap();
        if !is_squeezed(&s) {
            assert_eq!(s, raw);
            return None;
        }
        assert_eq!(unsqueeze(&s).unwrap(), raw);
        assert!(cipher::blocks(s.len()) < cipher::blocks(raw.len()) || is_squeezed(raw));
        Some(s[1])
    }

    /// A small repeatable generator, so the damage tests try the same things every run.
    fn dice(mut seed: u64) -> impl FnMut() -> u64 {
        move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        }
    }

    const NOTE: &str = "all work and no play makes jack a dull boy. ";

    #[test]
    fn letters() {
        assert_eq!(short_kind(b'S'), Some((false, (3, 2))));
        assert_eq!(short_kind(b'V'), Some((false, (0, 0))));
        assert_eq!(short_kind(b'W'), Some((true, (3, 2))));
        assert_eq!(short_kind(b'Z'), Some((true, (0, 0))));
        assert_eq!(short_kind(b'L'), None);
        assert_eq!(short_kind(b'R'), None);
        assert_eq!(short_kind(b'['), None);
        assert_eq!(MODELS.map(props), [93, 3, 2, 0]);
        // None of them is the `h` a file payload has in this place.
        assert!(!is_squeezed(&crate::payload::FILE_MAGIC));
        for letter in [b'L', b'S', b'T', b'U', b'V', b'W', b'X', b'Y', b'Z'] {
            assert!(is_squeezed(&[0, letter, 1, 2, 3]));
        }
        assert!(!is_squeezed(&[0, b'K']) && !is_squeezed(&[0, b'R']) && !is_squeezed(b"Sure") && !is_squeezed(&[0]));
    }

    #[test]
    fn lossless_across_shapes() {
        let text = "The quick brown fox jumps over the lazy dog. ".repeat(200);
        assert!(matches!(there_and_back(text.as_bytes()), Some(b'S'..=b'Z')));
        assert_eq!(there_and_back(b"hi"), None);
        assert_eq!(there_and_back(b""), None);
        assert_eq!(there_and_back(&noise(5000)), None);
        assert_eq!(there_and_back(&vec![0u8; 3 << 20]), Some(b'L'));
        // Longer than the window, so matches have to reach across it.
        let mut big = text.clone().into_bytes();
        while big.len() < (9 << 20) {
            big.extend_from_slice(&noise(64));
            big.extend_from_slice(text.as_bytes());
        }
        assert_eq!(there_and_back(&big), Some(b'L'));
    }

    #[test]
    fn every_form_round_trips_at_every_size() {
        let source = NOTE.repeat(1600).into_bytes();
        let sizes = (1..200).chain([255, 256, 257, 4095, 4096, 4097, 16383, 16384, 16385, 65535, 65536]);
        for len in sizes {
            let raw = &source[..len];
            assert_eq!(unsqueeze(&long_of(raw)).unwrap(), raw, "long {len}");
            for (at, model) in MODELS.into_iter().enumerate() {
                for primed in [false, true] {
                    let stream = encode(raw, false, model, primed.then_some(WORDS.as_bytes()));
                    let name = format!("{} model {at} len {len}", if primed { "words" } else { "short" });
                    // Too short to shrink means a stored chunk, which only the long form holds.
                    let Some(packed) = pack_short(primed, at, len, &stream) else {
                        assert!(len < 64, "{name} should have packed");
                        continue;
                    };
                    assert_eq!(unsqueeze(&packed).unwrap(), raw, "{name}");
                    // The stream minus what can be rebuilt: 6 bytes of header and a byte at each end.
                    let varint = if len < 128 { 1 } else if len < 16384 { 2 } else { 3 };
                    assert_eq!(packed.len(), 2 + varint + stream.len() - 8, "{name}");
                }
            }
        }
        // One past what a single chunk can describe.
        let raw = &source[..SHORT_MAX + 1];
        assert_eq!(pack_short(false, 0, raw.len(), &encode(raw, false, STOCK, None)), None);
        assert_eq!(there_and_back(raw), Some(b'L'));
    }

    #[test]
    fn packed_data_is_left_alone() {
        let raw = noise(3 << 20);
        assert_eq!(there_and_back(&raw), None);
        assert_eq!(there_and_back(&noise(20_000)), None);
    }

    #[test]
    fn a_payload_that_starts_with_a_marker_is_always_wrapped() {
        for letter in [b'L', b'S', b'T', b'U', b'V', b'W', b'X', b'Y', b'Z'] {
            for tail in [Vec::new(), noise(64), NOTE.repeat(9).into_bytes()] {
                let raw = [&[NUL, letter][..], &tail].concat();
                assert!(there_and_back(&raw).is_some(), "{} + {} bytes", letter as char, tail.len());
            }
        }
    }

    #[test]
    fn damage_is_caught_in_every_form() {
        let mut next = dice(0x9e37_79b9_7f4a_7c15);
        // A plain sentence, and one that leans on the word list.
        for raw in [NOTE.repeat(30), "Let me know when you get home, and thank you for today. ".repeat(4)] {
            let raw = raw.into_bytes();
            for good in [long_of(&raw), short_of(&raw), primed_of(&raw)] {
                let form = good[1] as char;
                assert_eq!(unsqueeze(&good).unwrap(), raw);

                for cut in 1..good.len() - 2 {
                    assert_eq!(unsqueeze(&good[..good.len() - cut]), Err(Damaged), "{form} cut {cut}");
                }
                for extra in 1..20 {
                    let mut longer = good.clone();
                    longer.extend((0..extra).map(|_| next() as u8));
                    assert_eq!(unsqueeze(&longer), Err(Damaged), "{form} {extra} more");
                }
                // What a damaged huH does to a payload: one whole AES block of it comes
                // out scrambled. (The first block holds the marker; scrambling that
                // makes the payload something else entirely, which is locker's test.)
                for block in 1..good.len().div_ceil(16) {
                    for round in 0..200 {
                        let mut bad = good.clone();
                        for b in bad.iter_mut().skip(block * 16).take(16) {
                            *b = next() as u8;
                        }
                        if bad != good {
                            assert_eq!(unsqueeze(&bad), Err(Damaged), "{form} block {block} round {round}");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn the_long_form_catches_every_flipped_bit() {
        let raw = NOTE.repeat(30).into_bytes();
        let good = long_of(&raw);
        for at in 2..good.len() {
            for bit in 0..8 {
                let mut bad = good.clone();
                bad[at] ^= 1 << bit;
                assert_eq!(unsqueeze(&bad), Err(Damaged), "bit {bit} at {at}");
            }
        }
    }

    #[test]
    fn a_stored_chunk_is_still_checked() {
        // Noise goes into LZMA2 as plain stored chunks, which carry no check of
        // their own; only the long form holds them, and its checksum covers them.
        let raw = noise(300);
        let good = long_of(&raw);
        assert_eq!(unsqueeze(&good).unwrap(), raw);
        let mut bad = good.clone();
        bad[100] ^= 0x01;
        assert_eq!(unsqueeze(&bad), Err(Damaged));
        assert_eq!(pack_short(false, 0, raw.len(), &encode(&raw, false, STOCK, None)), None);
    }

    #[test]
    fn a_forged_length_is_refused() {
        let mut forged = vec![NUL, LONG];
        push_len(&mut forged, MAX + 1);
        forged.extend_from_slice(&[0u8; 16]);
        assert_eq!(unsqueeze(&forged), Err(Damaged));

        let raw = vec![7u8; 5000];
        for good in [long_of(&raw), short_of(&raw), primed_of(&raw)] {
            for claim in [0usize, 1, 4000, 4999, 5001, 70_000] {
                let mut forged = good[..2].to_vec();
                push_len(&mut forged, claim);
                forged.extend_from_slice(&good[4..]);
                assert_eq!(unsqueeze(&forged), Err(Damaged), "{} claims {claim}", good[1] as char);
            }
        }
    }

    #[test]
    fn rubbish_never_gets_through() {
        // A wrong key that happens to pad correctly hands unlock rubbish; it has to
        // be refused quietly, whatever it looks like.
        let mut next = dice(0x2545_f491_4f6c_dd1d);
        let raw = NOTE.repeat(12).into_bytes();
        let goods = [long_of(&raw), short_of(&raw), primed_of(&raw)];
        let mut refused = 0u32;
        for round in 0..60_000u32 {
            let good = &goods[round as usize % goods.len()];
            let mut bad = good.clone();
            match next() % 4 {
                // Rubbish after a real marker.
                0 => {
                    let n = 3 + (next() % 200) as usize;
                    bad.truncate(1);
                    bad.push(b"LSTUVWXYZ"[next() as usize % 9]);
                    bad.extend((0..n).map(|_| next() as u8));
                }
                // A real payload with one AES block of it scrambled.
                1 => {
                    let block = 1 + next() as usize % (bad.len().div_ceil(16) - 1);
                    for b in bad.iter_mut().skip(block * 16).take(16) {
                        *b = next() as u8;
                    }
                }
                // Cut short, or with rubbish on the end.
                2 => {
                    let keep = 2 + (next() as usize % (bad.len() - 2));
                    bad.truncate(keep);
                }
                _ => bad.extend((0..1 + next() % 40).map(|_| next() as u8)),
            }
            match unsqueeze(&bad) {
                Err(Damaged) => refused += 1,
                Ok(out) => assert!(bad == *good && out == raw, "round {round} let rubbish through"),
            }
        }
        assert!(refused > 59_000, "only {refused} refused");
    }

    #[test]
    fn forms_made_before_still_open() {
        // tests/squeezed.json holds one payload for each marker letter, made by the
        // build that introduced them. It is what freezes the forms and the word list.
        let fixtures: serde_json::Value =
            serde_json::from_str(include_str!("../tests/squeezed.json")).unwrap();
        let mut letters = Vec::new();
        for case in fixtures["payloads"].as_array().unwrap() {
            let payload = crate::b64::decode(case["payload"].as_str().unwrap()).unwrap();
            let letter = case["letter"].as_str().unwrap().as_bytes()[0];
            assert_eq!(payload[..2], [NUL, letter]);
            assert_eq!(
                unsqueeze(&payload).unwrap(),
                case["text"].as_str().unwrap().as_bytes(),
                "form {}",
                letter as char
            );
            letters.push(letter);
        }
        assert_eq!(letters, b"LSTUVWXYZ");
    }

    #[test]
    fn length_prefix_round_trips() {
        for n in [0usize, 1, 127, 128, 16_383, 16_384, MAX] {
            let mut v = Vec::new();
            push_len(&mut v, n);
            assert_eq!(read_len(&v), Some((n, v.len())));
        }
        // Five bytes can spell more than 32 bits; that is a number here, not a wrap to zero.
        assert_eq!(read_len(&[0x80, 0x80, 0x80, 0x80, 0x10]).map(|(n, _)| n as u64), usize::try_from(1u64 << 32).ok().map(|n| n as u64));
        assert_eq!(unsqueeze(&[NUL, LONG, 0x80, 0x80, 0x80, 0x80, 0x10, 0, 0, 0, 0, 0]), Err(Damaged));
        assert_eq!(read_len(&[0x80, 0x80, 0x80, 0x80, 0x80, 0x01]), None);
    }
}
