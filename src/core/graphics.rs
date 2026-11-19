//! Real pictures on terminals that can show them, through the Kitty graphics protocol.
//!
//! Widgets ask for a picture in an area during their render, the same way they ask for the
//! cursor. After the frame's cells are written, the terminal compares this frame's placements
//! with the last frame's and sends only what changed: pixels the terminal has not seen yet,
//! placements that are new or have moved, and deletions for the ones that are gone.

use std::cell::{Cell as StdCell, RefCell};
use std::collections::HashSet;
use std::io::{self, Write};

use crossterm::{cursor::MoveTo, queue};

use crate::{Area, Picture};

// The most base64 bytes one escape sequence may carry.
const CHUNK: usize = 4096;

/// A way of putting pictures on screen that a terminal understands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Protocol {
    /// The Kitty graphics protocol: Kitty, WezTerm, Ghostty and Konsole.
    Kitty,
}

// A picture asked for this frame.
#[derive(Debug, Clone)]
pub(crate) struct Placement {
    // The cells the picture is scaled to fill, in screen coordinates.
    pub area: Area,
    pub picture: Picture,
    // Identifies the pixels, so a picture already sent is not sent again.
    pub id: u32,
    // The part of the picture shown, in pixels, as x, y, width and height. `None` shows it all.
    pub crop: Option<(u16, u16, u16, u16)>,
}

thread_local! {
    static PLACEMENTS: RefCell<Vec<Placement>> = const { RefCell::new(Vec::new()) };
    // Set when a terminal is set up, and cleared again when it is restored.
    static SUPPORT: StdCell<Option<Protocol>> = const { StdCell::new(None) };
    // Set when a terminal is set up and on every resize.
    static CELL_PX: StdCell<Option<(u16, u16)>> = const { StdCell::new(None) };
}

/// Pictures drawn by the terminal itself, at full resolution, rather than as characters.
///
/// Most applications never use this directly: [`Image`](crate::Image) with
/// [`PixelMode::Graphics`](crate::PixelMode::Graphics) does it for them, and falls back to
/// [`HalfBlock`](crate::PixelMode::HalfBlock) on terminals that cannot show pictures.
///
/// [`cell_pixels`](Graphics::cell_pixels) is the useful part for drawing crisp charts: render
/// the picture at exactly the pixel size of the cells it will fill, and nothing gets scaled.
///
/// ```
/// use auxior::Graphics;
///
/// // Outside a running app nothing is set up, so there is nothing to ask.
/// assert!(!Graphics::supported());
/// assert_eq!(Graphics::cell_pixels(), None);
/// ```
pub struct Graphics;

impl Graphics {
    /// Whether the terminal can show pictures. Decided once, when the terminal is set up.
    pub fn supported() -> bool {
        Self::protocol().is_some()
    }

    /// How many pixels wide and tall one cell is, if the terminal says.
    ///
    /// Updated on every resize, since changing the font size changes it.
    pub fn cell_pixels() -> Option<(u16, u16)> {
        CELL_PX.with(StdCell::get)
    }

    /// Shows `picture` scaled to fill `area`, in screen coordinates, for this frame.
    ///
    /// Asking does nothing visible on a terminal that cannot show pictures, so check
    /// [`supported`](Graphics::supported) first and draw something else when it is not.
    pub fn place(area: Area, picture: &Picture) {
        Self::place_cropped(area, picture, None);
    }

    pub(crate) fn place_cropped(area: Area, picture: &Picture, crop: Option<(u16, u16, u16, u16)>) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let placement = Placement {
            area,
            picture: picture.clone(),
            id: picture.graphics_id(),
            crop,
        };
        PLACEMENTS.with(|placements| placements.borrow_mut().push(placement));
    }

    pub(crate) fn protocol() -> Option<Protocol> {
        SUPPORT.try_with(StdCell::get).ok().flatten()
    }

    pub(crate) fn set_protocol(protocol: Option<Protocol>) {
        // `try_with`, because this also runs from the panic hook.
        let _ = SUPPORT.try_with(|support| support.set(protocol));
    }

    pub(crate) fn set_cell_pixels(pixels: Option<(u16, u16)>) {
        let _ = CELL_PX.try_with(|cell| cell.set(pixels));
    }

    // Reads the cell size from the terminal.
    pub(crate) fn refresh_cell_pixels() {
        let pixels = crossterm::terminal::window_size()
            .ok()
            .and_then(|size| cell_pixels_from(size.width, size.height, size.columns, size.rows));
        Self::set_cell_pixels(pixels);
    }

    // Forgets last frame's placements, called once per frame before anything draws.
    pub(crate) fn begin_frame() {
        PLACEMENTS.with(|placements| placements.borrow_mut().clear());
    }

    // This frame's placements, leaving none behind.
    pub(crate) fn take() -> Vec<Placement> {
        PLACEMENTS.with(|placements| std::mem::take(&mut *placements.borrow_mut()))
    }

    // This frame's placements, leaving them in place.
    pub(crate) fn peek() -> Vec<Placement> {
        PLACEMENTS.with(|placements| placements.borrow().clone())
    }
}

fn cell_pixels_from(width: u16, height: u16, columns: u16, rows: u16) -> Option<(u16, u16)> {
    if width == 0 || height == 0 || columns == 0 || rows == 0 {
        return None;
    }
    let across = width / columns;
    let down = height / rows;
    (across > 0 && down > 0).then_some((across, down))
}

// Works out from the environment whether the terminal speaks a graphics protocol.
pub(crate) fn detect(var: impl Fn(&str) -> Option<String>) -> Option<Protocol> {
    match var("AUXIOR_GRAPHICS").as_deref().map(str::trim) {
        Some(value) if value.eq_ignore_ascii_case("kitty") => return Some(Protocol::Kitty),
        Some(value) if value.eq_ignore_ascii_case("off") => return None,
        _ => {}
    }

    // tmux swallows the sequences unless each one is wrapped for passthrough.
    if var("TMUX").is_some() {
        return None;
    }

    let term = var("TERM").unwrap_or_default();
    let program = var("TERM_PROGRAM").unwrap_or_default();
    let kitty_like = var("KITTY_WINDOW_ID").is_some()
        || term.contains("kitty")
        || program == "WezTerm"
        || program == "ghostty"
        || var("GHOSTTY_RESOURCES_DIR").is_some()
        || var("KONSOLE_VERSION").is_some();

    kitty_like.then_some(Protocol::Kitty)
}

pub(crate) fn detect_from_env() -> Option<Protocol> {
    detect(|name| std::env::var(name).ok())
}

// 32-bit FNV-1a over the size and the pixels, never zero, since the protocol reserves zero.
pub(crate) fn picture_id(width: u16, height: u16, rgb: &[u8]) -> u32 {
    let mut hash: u32 = 0x811c_9dc5;
    let mut feed = |byte: u8| {
        hash ^= u32::from(byte);
        hash = hash.wrapping_mul(0x0100_0193);
    };
    width.to_le_bytes().into_iter().for_each(&mut feed);
    height.to_le_bytes().into_iter().for_each(&mut feed);
    rgb.iter().copied().for_each(&mut feed);
    hash.max(1)
}

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

// Standard base64, with padding.
pub(crate) fn base64(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let n = u32::from(b[0]) << 16 | u32::from(b[1]) << 8 | u32::from(b[2]);
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(ALPHABET[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

// Sends the pixels, split into chunks the terminal will accept.
pub(crate) fn write_transmit<W: Write>(
    writer: &mut W,
    picture: &Picture,
    id: u32,
) -> io::Result<()> {
    let (width, height) = picture.size();
    let encoded = base64(picture.rgb_bytes());
    // Every chunk but the last is a whole multiple of four, as the protocol requires.
    let chunks: Vec<&[u8]> = encoded.as_bytes().chunks(CHUNK).collect();
    let last = chunks.len().saturating_sub(1);

    for (index, chunk) in chunks.iter().enumerate() {
        let more = u8::from(index != last);
        if index == 0 {
            write!(
                writer,
                "\x1b_Ga=t,f=24,s={width},v={height},i={id},q=2,m={more};"
            )?;
        } else {
            // Repeating `q` keeps every terminal quiet, whichever chunk it reads it from.
            write!(writer, "\x1b_Gm={more},q=2;")?;
        }
        writer.write_all(chunk)?;
        writer.write_all(b"\x1b\\")?;
    }
    Ok(())
}

// A small non-zero number for a spot on screen, so placing the same picture there again
// replaces what is there rather than stacking another copy.
fn slot(area: Area) -> u32 {
    (u32::from(area.x) << 16 | u32::from(area.y))
        .wrapping_add(1)
        .max(1)
}

pub(crate) fn write_delete_all<W: Write>(writer: &mut W) -> io::Result<()> {
    writer.write_all(b"\x1b_Ga=d,d=A,q=2\x1b\\")
}

// What was put where, compared frame to frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Placed {
    area: Area,
    id: u32,
    crop: Option<(u16, u16, u16, u16)>,
}

/// What the terminal is showing, so each frame sends only what changed.
#[derive(Debug, Default)]
pub(crate) struct GraphicsSync {
    placed: Vec<Placed>,
    sent: HashSet<u32>,
    // Wipe everything before the next sync: on the first frame and after a resize.
    wipe: bool,
}

impl GraphicsSync {
    pub(crate) fn new() -> Self {
        Self {
            wipe: true,
            ..Self::default()
        }
    }

    /// Wipes everything on the next sync and sends it all again.
    pub(crate) fn invalidate(&mut self) {
        self.wipe = true;
    }

    /// Where each picture went last frame, and which pixels.
    pub(crate) fn placed(&self) -> Vec<(Area, u32)> {
        self.placed.iter().map(|p| (p.area, p.id)).collect()
    }

    pub(crate) fn sync<W: Write>(
        &mut self,
        writer: &mut W,
        placements: &[Placement],
    ) -> io::Result<()> {
        if self.wipe {
            write_delete_all(writer)?;
            self.placed.clear();
            self.sent.clear();
            self.wipe = false;
        }

        let mut now: Vec<Placed> = Vec::with_capacity(placements.len());
        for p in placements {
            let placed = Placed {
                area: p.area,
                id: p.id,
                crop: p.crop,
            };
            if !now.contains(&placed) {
                now.push(placed);
            }
        }

        for old in &self.placed {
            if !now.contains(old) {
                write!(
                    writer,
                    "\x1b_Ga=d,d=i,i={},p={},q=2\x1b\\",
                    old.id,
                    slot(old.area)
                )?;
            }
        }

        for placed in &now {
            if self.placed.contains(placed) {
                continue;
            }
            if !self.sent.contains(&placed.id) {
                // The first placement with these pixels carries them.
                let Some(p) = placements.iter().find(|p| p.id == placed.id) else {
                    continue;
                };
                write_transmit(writer, &p.picture, placed.id)?;
                self.sent.insert(placed.id);
            }
            queue!(writer, MoveTo(placed.area.x, placed.area.y))?;
            write!(
                writer,
                "\x1b_Ga=p,i={},p={},c={},r={},",
                placed.id,
                slot(placed.area),
                placed.area.width,
                placed.area.height
            )?;
            if let Some((x, y, w, h)) = placed.crop {
                write!(writer, "x={x},y={y},w={w},h={h},")?;
            }
            writer.write_all(b"C=1,q=2\x1b\\")?;
        }

        // Pixels nothing shows any more are freed, or a live chart that changes every minute
        // would fill the terminal's memory with old ones.
        let mut gone: Vec<u32> = self
            .sent
            .iter()
            .copied()
            .filter(|id| !now.iter().any(|p| p.id == *id))
            .collect();
        gone.sort_unstable();
        for id in gone {
            write!(writer, "\x1b_Ga=d,d=I,i={id},q=2\x1b\\")?;
            self.sent.remove(&id);
        }

        self.placed = now;
        Ok(())
    }
}

// Test cases
#[cfg(test)]
mod tests {
    use super::*;

    fn picture(width: u16, height: u16, seed: u8) -> Picture {
        Picture::from_fn(width, height, |x, y| {
            (seed, (x % 256) as u8, (y % 256) as u8)
        })
        .unwrap()
    }

    fn placement(area: Area, picture: &Picture) -> Placement {
        Placement {
            area,
            picture: picture.clone(),
            id: picture.graphics_id(),
            crop: None,
        }
    }

    fn sync(state: &mut GraphicsSync, placements: &[Placement]) -> String {
        let mut out = Vec::new();
        state.sync(&mut out, placements).unwrap();
        String::from_utf8(out).unwrap()
    }

    // The sync after the first, so the opening wipe is out of the way.
    fn settled() -> GraphicsSync {
        let mut state = GraphicsSync::new();
        sync(&mut state, &[]);
        state
    }

    fn sequences(output: &str) -> Vec<&str> {
        output
            .split("\x1b_G")
            .skip(1)
            .map(|seq| seq.split("\x1b\\").next().unwrap())
            .collect()
    }

    #[test]
    fn base64_matches_known_vectors() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foob"), "Zm9vYg==");
        assert_eq!(base64(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
        assert_eq!(base64(&[0xff, 0xfe, 0xfd]), "//79");
    }

    #[test]
    fn a_small_picture_goes_in_one_chunk() {
        let pic = picture(2, 2, 7);
        let mut out = Vec::new();
        write_transmit(&mut out, &pic, 42).unwrap();
        let output = String::from_utf8(out).unwrap();

        let seqs = sequences(&output);
        assert_eq!(seqs.len(), 1, "{output:?}");
        assert!(
            seqs[0].starts_with("a=t,f=24,s=2,v=2,i=42,q=2,m=0;"),
            "{output:?}"
        );
        assert!(seqs[0].ends_with(&base64(pic.rgb_bytes())));
    }

    #[test]
    fn a_large_picture_is_split_into_chunks() {
        // 3 * 3000 bytes encode to 12000 base64 bytes: three chunks.
        let pic = picture(100, 30, 1);
        let mut out = Vec::new();
        write_transmit(&mut out, &pic, 9).unwrap();
        let output = String::from_utf8(out).unwrap();
        let seqs = sequences(&output);

        assert_eq!(seqs.len(), 3);
        assert!(seqs[0].starts_with("a=t,f=24,s=100,v=30,i=9,q=2,m=1;"));
        assert!(seqs[1].starts_with("m=1"));
        assert!(!seqs[1].contains("a=") && !seqs[1].contains("i="));
        assert!(seqs[2].starts_with("m=0"));
        assert!(!seqs[2].contains("s="));

        let payload: String = seqs
            .iter()
            .map(|seq| seq.split_once(';').unwrap().1)
            .collect();
        assert_eq!(payload.len(), 12000);
        assert_eq!(seqs[0].split_once(';').unwrap().1.len(), CHUNK);
        assert_eq!(payload, base64(pic.rgb_bytes()));
    }

    #[test]
    fn ids_follow_the_pixels() {
        let a = picture(4, 4, 1);
        let same = picture(4, 4, 1);
        let other = picture(4, 4, 2);
        assert_eq!(a.graphics_id(), same.graphics_id());
        assert_ne!(a.graphics_id(), other.graphics_id());
        assert_ne!(picture_id(2, 8, &[0; 48]), picture_id(8, 2, &[0; 48]));
        assert_ne!(a.graphics_id(), 0);
    }

    #[test]
    fn the_first_sync_wipes_everything() {
        let mut state = GraphicsSync::new();
        assert_eq!(sync(&mut state, &[]), "\x1b_Ga=d,d=A,q=2\x1b\\");
        assert_eq!(sync(&mut state, &[]), "");
    }

    #[test]
    fn a_new_picture_is_sent_once_then_placed() {
        let mut state = settled();
        let pic = picture(3, 3, 1);
        let area = Area::new(2, 1, 4, 2);
        let output = sync(&mut state, &[placement(area, &pic)]);
        let seqs = sequences(&output);

        assert_eq!(seqs.len(), 2, "{output:?}");
        assert!(seqs[0].starts_with("a=t,"));
        assert_eq!(
            seqs[1],
            format!(
                "a=p,i={},p={},c=4,r=2,C=1,q=2",
                pic.graphics_id(),
                slot(area)
            )
        );
        // The move to the area's corner comes right before the placement.
        let place_at = output.find("\x1b_Ga=p").unwrap();
        assert!(output[..place_at].ends_with("\x1b[2;3H"), "{output:?}");
    }

    #[test]
    fn the_same_frame_twice_sends_nothing() {
        let mut state = settled();
        let pic = picture(3, 3, 1);
        let frame = [placement(Area::new(0, 0, 4, 2), &pic)];
        sync(&mut state, &frame);
        assert_eq!(sync(&mut state, &frame), "");
    }

    #[test]
    fn a_moved_picture_is_deleted_and_placed_without_resending() {
        let mut state = settled();
        let pic = picture(3, 3, 1);
        let id = pic.graphics_id();
        let from = Area::new(0, 0, 4, 2);
        let to = Area::new(5, 3, 4, 2);
        sync(&mut state, &[placement(from, &pic)]);

        let output = sync(&mut state, &[placement(to, &pic)]);
        let seqs = sequences(&output);
        assert_eq!(seqs.len(), 2, "{output:?}");
        assert_eq!(seqs[0], format!("a=d,d=i,i={id},p={},q=2", slot(from)));
        assert!(seqs[1].starts_with(&format!("a=p,i={id},p={}", slot(to))));
        assert!(!output.contains("a=t"));
    }

    #[test]
    fn a_dropped_picture_is_deleted_and_freed() {
        let mut state = settled();
        let pic = picture(3, 3, 1);
        let id = pic.graphics_id();
        let area = Area::new(1, 1, 2, 2);
        sync(&mut state, &[placement(area, &pic)]);

        let output = sync(&mut state, &[]);
        assert_eq!(
            sequences(&output),
            vec![
                format!("a=d,d=i,i={id},p={},q=2", slot(area)),
                format!("a=d,d=I,i={id},q=2"),
            ]
        );
        assert!(state.sent.is_empty());
    }

    #[test]
    fn a_replaced_picture_frees_the_old_pixels() {
        let mut state = settled();
        let area = Area::new(0, 0, 4, 4);
        let old = picture(3, 3, 1);
        let new = picture(3, 3, 2);
        sync(&mut state, &[placement(area, &old)]);

        let output = sync(&mut state, &[placement(area, &new)]);
        let seqs = sequences(&output);
        assert!(seqs[0].starts_with(&format!("a=d,d=i,i={}", old.graphics_id())));
        assert!(seqs[1].starts_with(&format!("a=t,f=24,s=3,v=3,i={}", new.graphics_id())));
        assert!(seqs[2].starts_with(&format!("a=p,i={}", new.graphics_id())));
        assert_eq!(seqs[3], format!("a=d,d=I,i={},q=2", old.graphics_id()));
    }

    #[test]
    fn one_picture_in_two_places_is_sent_once() {
        let mut state = settled();
        let pic = picture(3, 3, 1);
        let output = sync(
            &mut state,
            &[
                placement(Area::new(0, 0, 2, 2), &pic),
                placement(Area::new(4, 0, 2, 2), &pic),
            ],
        );
        assert_eq!(output.matches("a=t").count(), 1);
        assert_eq!(output.matches("a=p").count(), 2);
    }

    #[test]
    fn invalidating_wipes_and_sends_everything_again() {
        let mut state = settled();
        let pic = picture(3, 3, 1);
        let frame = [placement(Area::new(0, 0, 4, 2), &pic)];
        sync(&mut state, &frame);

        state.invalidate();
        let output = sync(&mut state, &frame);
        let seqs = sequences(&output);
        assert_eq!(seqs[0], "a=d,d=A,q=2");
        assert!(seqs[1].starts_with("a=t,"));
        assert!(seqs[2].starts_with("a=p,"));
        assert_eq!(seqs.len(), 3);
    }

    #[test]
    fn a_crop_is_sent_with_the_placement() {
        let mut state = settled();
        let pic = picture(10, 10, 1);
        let mut p = placement(Area::new(0, 0, 2, 2), &pic);
        p.crop = Some((1, 2, 3, 4));
        let output = sync(&mut state, &[p]);
        assert!(output.contains(",x=1,y=2,w=3,h=4,C=1"), "{output:?}");
    }

    fn env<'a>(vars: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |name| {
            vars.iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| value.to_string())
        }
    }

    #[test]
    fn detection_follows_the_environment() {
        let kitty = Some(Protocol::Kitty);
        assert_eq!(detect(env(&[])), None);
        assert_eq!(detect(env(&[("TERM", "xterm-256color")])), None);
        assert_eq!(detect(env(&[("KITTY_WINDOW_ID", "1")])), kitty);
        assert_eq!(detect(env(&[("TERM", "xterm-kitty")])), kitty);
        assert_eq!(detect(env(&[("TERM_PROGRAM", "WezTerm")])), kitty);
        assert_eq!(detect(env(&[("TERM_PROGRAM", "ghostty")])), kitty);
        assert_eq!(detect(env(&[("TERM_PROGRAM", "vscode")])), None);
        assert_eq!(detect(env(&[("GHOSTTY_RESOURCES_DIR", "/usr")])), kitty);
        assert_eq!(detect(env(&[("KONSOLE_VERSION", "240800")])), kitty);
    }

    #[test]
    fn tmux_turns_detection_off() {
        assert_eq!(
            detect(env(&[("TERM", "xterm-kitty"), ("TMUX", "/tmp/tmux,1,0")])),
            None
        );
    }

    #[test]
    fn the_override_beats_everything() {
        assert_eq!(
            detect(env(&[("AUXIOR_GRAPHICS", "kitty"), ("TMUX", "x")])),
            Some(Protocol::Kitty)
        );
        assert_eq!(
            detect(env(&[("AUXIOR_GRAPHICS", "kitty")])),
            Some(Protocol::Kitty)
        );
        assert_eq!(
            detect(env(&[("AUXIOR_GRAPHICS", "off"), ("KITTY_WINDOW_ID", "1")])),
            None
        );
    }

    #[test]
    fn cell_pixels_divide_the_window() {
        assert_eq!(cell_pixels_from(800, 480, 80, 24), Some((10, 20)));
        assert_eq!(cell_pixels_from(0, 0, 80, 24), None);
        assert_eq!(cell_pixels_from(800, 480, 0, 24), None);
    }

    #[test]
    fn placements_last_one_frame() {
        let pic = picture(2, 2, 1);
        Graphics::begin_frame();
        Graphics::place(Area::new(0, 0, 3, 3), &pic);
        Graphics::place(Area::new(0, 0, 0, 3), &pic);
        assert_eq!(Graphics::peek().len(), 1);
        Graphics::begin_frame();
        assert!(Graphics::take().is_empty());
    }
}
