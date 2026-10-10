//! The last-resort interface fonts: faces already installed on this machine.
//!
//! The embedded faces (Inter, egui's defaults, craft-fonts) come first in every family; these
//! only draw characters none of them has, such as a Hebrew, Korean or Thai file name (#586, #407,
//! #664). One face is loaded per script that nothing loaded so far covers, so a broad face (Arial
//! Unicode, DejaVu) found early spares the later ones. They are read at runtime and never embedded
//! or shipped (AGENTS.md §1.4), and `PDFCRAFT_SYSTEM_FONTS=0` turns them off (published
//! screenshots do).

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use egui::FontData;

/// Larger files are not read: a font path is still untrusted input.
const MAX_BYTES: u64 = 32 << 20;
/// All fallback faces together stay under this, so a machine full of large CJK collections
/// doesn't cost the interface hundreds of megabytes.
const MAX_TOTAL_BYTES: u64 = 96 << 20;
/// Faces tried in a collection (`.ttc`).
const MAX_FACES: u32 = 16;

/// A script the interface may need to draw, the character a face must map to cover it, and the
/// installed files to try for it, best first (paths as [`candidates`] resolves them).
struct Script {
    tag: &'static str,
    probe: char,
    mac: &'static [&'static str],
    windows: &'static [&'static str],
    unix: &'static [&'static str],
}

/// In order: the earlier a script, the likelier its face also covers later ones.
const SCRIPTS: &[Script] = &[
    Script {
        tag: "Arab",
        probe: '\u{0627}',
        mac: &["SFArabic.ttf", "GeezaPro.ttc"],
        windows: &["segoeui.ttf", "tahoma.ttf", "arial.ttf"],
        unix: &["noto/NotoSansArabic-Regular.ttf", "dejavu/DejaVuSans.ttf"],
    },
    Script {
        tag: "Hebr",
        probe: '\u{05D0}',
        mac: &["SFHebrew.ttf", "ArialHB.ttc"],
        windows: &["segoeui.ttf", "arial.ttf"],
        unix: &["noto/NotoSansHebrew-Regular.ttf", "dejavu/DejaVuSans.ttf"],
    },
    Script {
        tag: "Armn",
        probe: '\u{0531}',
        mac: &["SFArmenian.ttf"],
        windows: &["segoeui.ttf"],
        unix: &["noto/NotoSansArmenian-Regular.ttf", "dejavu/DejaVuSans.ttf"],
    },
    Script {
        tag: "Geor",
        probe: '\u{10D0}',
        mac: &["SFGeorgian.ttf"],
        windows: &["segoeui.ttf"],
        unix: &["noto/NotoSansGeorgian-Regular.ttf", "dejavu/DejaVuSans.ttf"],
    },
    // Arial Unicode covers Han, kana and Hangul at once; the Windows and Noto faces are per region.
    Script {
        tag: "Hani",
        probe: '\u{4E2D}',
        mac: &["Supplemental/Arial Unicode.ttf", "Hiragino Sans GB.ttc"],
        windows: &["msyh.ttc", "YuGothR.ttc", "msgothic.ttc", "simsun.ttc"],
        unix: &["noto-cjk/NotoSansCJK-Regular.ttc", "noto/NotoSansCJK-Regular.ttc", "wqy/wqy-microhei.ttc", "wqy-microhei/wqy-microhei.ttc"],
    },
    Script {
        tag: "Kana",
        probe: '\u{3042}',
        mac: &["Supplemental/Arial Unicode.ttf", "Hiragino Sans GB.ttc"],
        windows: &["YuGothR.ttc", "msgothic.ttc", "msyh.ttc"],
        unix: &["noto-cjk/NotoSansCJK-Regular.ttc", "noto/NotoSansCJK-Regular.ttc"],
    },
    Script {
        tag: "Hang",
        probe: '\u{AC00}',
        mac: &["Supplemental/Arial Unicode.ttf"],
        windows: &["malgun.ttf", "gulim.ttc"],
        unix: &["noto-cjk/NotoSansCJK-Regular.ttc", "noto/NotoSansCJK-Regular.ttc", "nanum/NanumGothic.ttf", "baekmuk/gulim.ttf"],
    },
    Script {
        tag: "Thai",
        probe: '\u{0E01}',
        mac: &["ThonburiUI.ttc", "Supplemental/Thonburi.ttc"],
        windows: &["LeelawUI.ttf", "tahoma.ttf"],
        unix: &["noto/NotoSansThai-Regular.ttf", "tlwg/Loma.ttf", "tlwg/Garuda.ttf"],
    },
    Script {
        tag: "Deva",
        probe: '\u{0915}',
        mac: &["Kohinoor.ttc", "Supplemental/Devanagari Sangam MN.ttc"],
        windows: &["Nirmala.ttf", "mangal.ttf"],
        unix: &["noto/NotoSansDevanagari-Regular.ttf", "lohit-devanagari/Lohit-Devanagari.ttf"],
    },
    Script {
        tag: "Beng",
        probe: '\u{0995}',
        mac: &["KohinoorBangla.ttc"],
        windows: &["Nirmala.ttf", "vrinda.ttf"],
        unix: &["noto/NotoSansBengali-Regular.ttf", "lohit-bengali/Lohit-Bengali.ttf"],
    },
    Script {
        tag: "Guru",
        probe: '\u{0A15}',
        mac: &["Supplemental/Gurmukhi Sangam MN.ttc", "Supplemental/Gurmukhi.ttf"],
        windows: &["Nirmala.ttf"],
        unix: &["noto/NotoSansGurmukhi-Regular.ttf"],
    },
    Script {
        tag: "Gujr",
        probe: '\u{0A95}',
        mac: &["KohinoorGujarati.ttc"],
        windows: &["Nirmala.ttf"],
        unix: &["noto/NotoSansGujarati-Regular.ttf"],
    },
    Script {
        tag: "Taml",
        probe: '\u{0B95}',
        mac: &["Supplemental/Tamil Sangam MN.ttc"],
        windows: &["Nirmala.ttf", "latha.ttf"],
        unix: &["noto/NotoSansTamil-Regular.ttf", "lohit-tamil/Lohit-Tamil.ttf"],
    },
    Script { tag: "Telu", probe: '\u{0C15}', mac: &["KohinoorTelugu.ttc"], windows: &["Nirmala.ttf"], unix: &["noto/NotoSansTelugu-Regular.ttf"] },
    Script {
        tag: "Knda",
        probe: '\u{0C95}',
        mac: &["NotoSansKannada.ttc", "Supplemental/Kannada Sangam MN.ttc"],
        windows: &["Nirmala.ttf"],
        unix: &["noto/NotoSansKannada-Regular.ttf"],
    },
    Script {
        tag: "Mlym",
        probe: '\u{0D15}',
        mac: &["Supplemental/Malayalam Sangam MN.ttc"],
        windows: &["Nirmala.ttf"],
        unix: &["noto/NotoSansMalayalam-Regular.ttf"],
    },
    Script {
        tag: "Sinh",
        probe: '\u{0D9A}',
        mac: &["Supplemental/Sinhala Sangam MN.ttc"],
        windows: &["Nirmala.ttf"],
        unix: &["noto/NotoSansSinhala-Regular.ttf"],
    },
    Script {
        tag: "Mymr",
        probe: '\u{1000}',
        mac: &["NotoSansMyanmar.ttc", "Supplemental/Myanmar Sangam MN.ttc"],
        windows: &["mmrtext.ttf"],
        unix: &["noto/NotoSansMyanmar-Regular.ttf"],
    },
    Script {
        tag: "Khmr",
        probe: '\u{1780}',
        mac: &["Supplemental/Khmer Sangam MN.ttf"],
        windows: &["LeelawUI.ttf"],
        unix: &["noto/NotoSansKhmer-Regular.ttf"],
    },
    Script {
        tag: "Laoo",
        probe: '\u{0E81}',
        mac: &["Supplemental/Lao Sangam MN.ttf"],
        windows: &["LeelawUI.ttf"],
        unix: &["noto/NotoSansLao-Regular.ttf"],
    },
    Script {
        tag: "Ethi",
        probe: '\u{1200}',
        mac: &["Supplemental/KefaIII.ttf"],
        windows: &["ebrima.ttf"],
        unix: &["noto/NotoSansEthiopic-Regular.ttf"],
    },
];

/// One installed fallback face: the script it was loaded for (`Hebr`, `Hani`…) and its data.
pub struct Fallback {
    pub tag: &'static str,
    pub data: Arc<FontData>,
}

/// The installed fallback faces for scripts `embedded` (the app's own faces) doesn't cover, read
/// once: later calls return the first result. Empty when turned off or nothing fits.
pub fn fallbacks(embedded: &[&FontData]) -> &'static [Fallback] {
    static CACHE: OnceLock<Vec<Fallback>> = OnceLock::new();
    CACHE.get_or_init(|| load(embedded))
}

fn load(embedded: &[&FontData]) -> Vec<Fallback> {
    if std::env::var_os("PDFCRAFT_SYSTEM_FONTS").is_some_and(|v| v == "0") {
        return Vec::new();
    }
    let mut loaded: Vec<(PathBuf, Fallback)> = Vec::new();
    let mut total: u64 = 0;
    for script in SCRIPTS {
        let covered = embedded.iter().any(|d| maps(d, script.probe)) || loaded.iter().any(|(_, f)| maps(&f.data, script.probe));
        if covered {
            continue;
        }
        for path in candidates(script) {
            // A file already loaded for another script doesn't have this one (checked above).
            if loaded.iter().any(|(p, _)| *p == path) {
                continue;
            }
            if let Some((data, size)) = read(&path, script.probe, MAX_TOTAL_BYTES.saturating_sub(total)) {
                log::info!("interface font fallback for {}: {}", script.tag, path.display());
                total = total.saturating_add(size);
                loaded.push((path, Fallback { tag: script.tag, data }));
                break;
            }
        }
    }
    loaded.into_iter().map(|(_, f)| f).collect()
}

/// The installed files to try for `script` on this platform, best first.
fn candidates(script: &Script) -> Vec<PathBuf> {
    if cfg!(windows) {
        let dir = std::env::var_os("WINDIR").or_else(|| std::env::var_os("SystemRoot")).map_or_else(|| PathBuf::from(r"C:\Windows"), PathBuf::from);
        script.windows.iter().map(|f| dir.join("Fonts").join(f)).collect()
    } else if cfg!(target_os = "macos") {
        script.mac.iter().map(|f| Path::new("/System/Library/Fonts").join(f)).collect()
    } else {
        // Distributions file the same font under different folders: `noto/X` is also tried as
        // `truetype/noto/X`, `opentype/noto/X` and `google-noto/X`.
        let mut out = Vec::new();
        for dir in ["/usr/share/fonts", "/usr/local/share/fonts"] {
            for f in script.unix {
                let (folder, file) = f.split_once('/').unwrap_or(("", f));
                let mut folders = vec![folder.to_owned(), format!("truetype/{folder}"), format!("opentype/{folder}"), format!("google-{folder}")];
                if folder == "dejavu" {
                    folders.extend(["TTF".to_owned(), "dejavu-sans-fonts".to_owned()]);
                }
                out.extend(folders.iter().map(|d| Path::new(dir).join(d).join(file)));
            }
        }
        out
    }
}

/// The face of `path` that maps `probe`, with the file's size, if the file is no larger than
/// [`MAX_BYTES`] or `budget`.
fn read(path: &Path, probe: char, budget: u64) -> Option<(Arc<FontData>, u64)> {
    let meta = std::fs::metadata(path).ok()?;
    if !meta.is_file() || meta.len() > MAX_BYTES || meta.len() > budget {
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    let size = u64::try_from(bytes.len()).ok()?;
    let index = face_with(&bytes, probe)?;
    let mut data = FontData::from_owned(bytes);
    data.index = index;
    Some((Arc::new(data), size))
}

/// Whether the face `data` names maps `c`.
fn maps(data: &FontData, c: char) -> bool {
    use skrifa::MetadataProvider as _;
    skrifa::FontRef::from_index(&data.font, data.index).is_ok_and(|font| font.charmap().map(c).is_some())
}

/// The first face of the file that parses and maps `c`. egui parses fonts with the same skrifa,
/// so a face accepted here is one it can load.
fn face_with(bytes: &[u8], c: char) -> Option<u32> {
    use skrifa::MetadataProvider as _;
    (0..MAX_FACES).find(|&index| skrifa::FontRef::from_index(bytes, index).is_ok_and(|font| font.charmap().map(c).is_some()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ARABIC: char = '\u{0627}';

    #[test]
    fn broken_and_missing_files_are_skipped() {
        assert_eq!(face_with(b"", ARABIC), None);
        assert_eq!(face_with(b"not a font at all", ARABIC), None);
        assert_eq!(face_with(&[0u8; 4096], ARABIC), None);
        assert!(read(Path::new("definitely/not/here.ttf"), ARABIC, MAX_TOTAL_BYTES).is_none());
        // A directory is not a font.
        assert!(read(&std::env::temp_dir(), ARABIC, MAX_TOTAL_BYTES).is_none());
        assert!(!maps(&FontData::from_static(b"not a font"), 'A'));
    }

    #[test]
    fn a_face_without_the_probe_is_rejected() {
        // Inter is Latin, Greek and Cyrillic only.
        let inter = include_bytes!("../../../assets/fonts/Inter-Regular.ttf");
        assert_eq!(face_with(inter, ARABIC), None);
        assert_eq!(face_with(inter, 'A'), Some(0));
        let data = FontData::from_static(inter);
        assert!(maps(&data, 'A'));
        assert!(!maps(&data, '\u{05D0}'));
    }

    #[test]
    fn candidates_are_absolute_font_files() {
        for script in SCRIPTS {
            let list = candidates(script);
            assert!(!list.is_empty(), "{}", script.tag);
            assert!(list.iter().all(|p| p.is_absolute() && p.extension().is_some_and(|e| e == "ttf" || e == "ttc")), "{}: {list:?}", script.tag);
        }
    }

    #[test]
    fn script_tags_are_distinct() {
        let mut tags: Vec<_> = SCRIPTS.iter().map(|s| s.tag).collect();
        tags.sort_unstable();
        tags.dedup();
        assert_eq!(tags.len(), SCRIPTS.len());
    }

    /// A face over the budget is skipped, so the total stays under [`MAX_TOTAL_BYTES`].
    #[test]
    fn the_budget_is_respected() {
        let inter = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/fonts/Inter-Regular.ttf");
        assert!(read(&inter, 'A', MAX_TOTAL_BYTES).is_some());
        assert!(read(&inter, 'A', 1).is_none());
    }
}
