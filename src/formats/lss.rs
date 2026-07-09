//! Import/export of LiveSplit's `.lss` XML format.
//!
//! Schema confirmed against a real file (`slaurent22/lss-tools`,
//! `example-splits/4ms.lss`), not from memory. Two conversions matter:
//!
//! - `Segment/SplitTimes/SplitTime` holds *cumulative* time-from-start per
//!   comparison; `Segment/BestSegmentTime` and `SegmentHistory/Time` hold
//!   *segment* (relative) times. Our `Split.comparisons` /
//!   `Split.segment_history` are all relative, so import/export convert
//!   between the two by tracking a running cumulative total per comparison.
//! - LiveSplit derives "Average Segments"/"Median Segments"/etc. itself from
//!   `SegmentHistory` at runtime — they're never written as `SplitTime`
//!   entries, and neither are ours on export.
//!
//! Known limitations (see the LSS_IMPORT_EXPORT plan): icons are
//! `.NET BinaryFormatter`-wrapped, not plain PNGs — import extracts the
//! embedded PNG bytes with a signature scan; export never embeds icons.
//! `AutoSplitterSettings` is ignored both ways. `AttemptCount`/
//! `AttemptHistory` fidelity is limited by the fact that `Run::attempts`
//! only counts completed runs, not resets.

use chrono::{DateTime, Duration, Utc};
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::collections::BTreeMap;
use std::path::Path;

use crate::core::split::{
    AttemptHistoryEntry, COMPARISON_BEST_SEGMENTS, Run, RunMetadata, RunVariable,
    SegmentHistoryEntry,
};

use super::time::{format_dotnet_timespan, parse_dotnet_timespan};

// ---------------------------------------------------------------------
// A minimal generic XML tree, used only for reading. Building this once
// up front is much less error-prone than hand-tracking a stack of
// in-progress fields against quick_xml's flat event stream, given how many
// elements in this schema share names across different nesting contexts
// (`RealTime`/`GameTime` appear under `Attempt`, `SplitTime`,
// `BestSegmentTime`, and `Time` alike).
// ---------------------------------------------------------------------

struct XmlNode {
    name: String,
    attrs: Vec<(String, String)>,
    children: Vec<XmlNode>,
    text: String,
}

impl XmlNode {
    fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }

    fn child(&self, name: &str) -> Option<&XmlNode> {
        self.children.iter().find(|c| c.name == name)
    }

    fn children_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a XmlNode> {
        self.children.iter().filter(move |c| c.name == name)
    }

    fn text_trim(&self) -> &str {
        self.text.trim()
    }

    fn child_time(&self, name: &str) -> Option<Duration> {
        self.child(name)
            .and_then(|n| parse_dotnet_timespan(n.text_trim()))
    }
}

fn parse_xml_tree(xml: &str) -> Result<XmlNode, String> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut stack: Vec<XmlNode> = vec![XmlNode {
        name: "#root".to_string(),
        attrs: Vec::new(),
        children: Vec::new(),
        text: String::new(),
    }];

    loop {
        let event = reader.read_event().map_err(|e| {
            format!(
                "XML parse error at position {}: {e}",
                reader.buffer_position()
            )
        })?;

        match event {
            Event::Start(e) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                let attrs = e
                    .attributes()
                    .filter_map(|a| a.ok())
                    .map(|a| {
                        let key = String::from_utf8_lossy(a.key.as_ref()).into_owned();
                        let value = a
                            .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                            .unwrap_or_default()
                            .into_owned();
                        (key, value)
                    })
                    .collect();
                stack.push(XmlNode {
                    name,
                    attrs,
                    children: Vec::new(),
                    text: String::new(),
                });
            }
            Event::Empty(e) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                let attrs = e
                    .attributes()
                    .filter_map(|a| a.ok())
                    .map(|a| {
                        let key = String::from_utf8_lossy(a.key.as_ref()).into_owned();
                        let value = a
                            .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                            .unwrap_or_default()
                            .into_owned();
                        (key, value)
                    })
                    .collect();
                stack
                    .last_mut()
                    .ok_or("Unbalanced XML tags")?
                    .children
                    .push(XmlNode {
                        name,
                        attrs,
                        children: Vec::new(),
                        text: String::new(),
                    });
            }
            Event::End(_) => {
                let node = stack
                    .pop()
                    .ok_or("Unbalanced XML tags (extra closing tag)")?;
                stack
                    .last_mut()
                    .ok_or("Unbalanced XML tags")?
                    .children
                    .push(node);
            }
            Event::Text(e) => {
                let decoded = e.decode().map_err(|e| format!("XML text error: {e}"))?;
                let text = quick_xml::escape::unescape(&decoded)
                    .map_err(|e| format!("XML text error: {e}"))?
                    .into_owned();
                if let Some(top) = stack.last_mut() {
                    top.text.push_str(&text);
                }
            }
            Event::CData(e) => {
                let text = String::from_utf8_lossy(&e.into_inner()).into_owned();
                if let Some(top) = stack.last_mut() {
                    top.text.push_str(&text);
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }

    let mut root_wrapper = stack.pop().ok_or("Empty XML document")?;
    if !stack.is_empty() {
        return Err("Unbalanced XML tags (unclosed element)".to_string());
    }
    root_wrapper
        .children
        .pop()
        .ok_or_else(|| "Empty XML document".to_string())
}

fn parse_dotnet_datetime(raw: &str) -> Option<DateTime<Utc>> {
    chrono::NaiveDateTime::parse_from_str(raw, "%m/%d/%Y %H:%M:%S")
        .ok()
        .map(|naive| naive.and_utc())
}

// ---------------------------------------------------------------------
// Import
// ---------------------------------------------------------------------

/// Result of importing a `.lss` file: the converted `Run`, plus the
/// LiveSplit format version the file declared (informational only — the
/// parser doesn't branch on it, see module docs).
pub struct ImportResult {
    pub run: Run,
    pub source_version: Option<String>,
}

/// Imports a LiveSplit `.lss` file into a `Run`. Icons embedded in the file
/// (if any could be extracted — see module docs) are written as PNGs into
/// `icons_dir`, with `Split::icon_path` set to `icons/<file>` relative to
/// wherever the caller ends up saving `split.json` next to `icons_dir`.
///
/// Deliberately does **not** branch on the declared LiveSplit version:
/// verified against 16 real `.lss` files spanning versions 1.0 through 1.8
/// (see the LSS_IMPORT_EXPORT plan notes), the schema drift that actually
/// occurs (e.g. `<Metadata><Variables>` splitting into
/// `<SpeedrunComVariables>`/`<CustomVariables>` in 1.8) is handled by
/// checking every known field name unconditionally rather than gating on a
/// version number — more robust than version-branching, since it doesn't
/// depend on us knowing exactly which version introduced which field.
pub fn import(path: &Path, icons_dir: &Path) -> Result<ImportResult, String> {
    let xml = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
    let xml = xml.trim_start_matches('\u{feff}');
    let root = parse_xml_tree(xml)?;

    if root.name != "Run" {
        return Err(format!(
            "Expected a <Run> root element, found <{}>",
            root.name
        ));
    }

    let source_version = root.attr("version").map(str::to_string);

    let game_name = root
        .child("GameName")
        .map(|n| n.text_trim().to_string())
        .unwrap_or_default();
    let category_name = root
        .child("CategoryName")
        .map(|n| n.text_trim().to_string())
        .unwrap_or_default();

    let segment_names: Vec<String> = root
        .child("Segments")
        .map(|segs| {
            segs.children_named("Segment")
                .map(|s| {
                    s.child("Name")
                        .map(|n| n.text_trim().to_string())
                        .unwrap_or_default()
                })
                .collect()
        })
        .unwrap_or_default();

    if segment_names.is_empty() {
        return Err("The .lss file has no segments".to_string());
    }
    let segment_name_refs: Vec<&str> = segment_names.iter().map(String::as_str).collect();

    let mut run = Run::new(&game_name, &category_name, &segment_name_refs);

    if let Some(offset) = root.child_time("Offset") {
        let secs = -offset.num_seconds();
        run.start_offset = if secs != 0 { Some(secs) } else { None };
    }

    if let Some(meta) = root.child("Metadata") {
        let mut metadata = RunMetadata::default();
        if let Some(id) = meta.child("Run").and_then(|n| n.attr("id"))
            && !id.is_empty()
        {
            metadata.speedrun_com_category_id = Some(id.to_string());
        }
        if let Some(platform) = meta.child("Platform").map(XmlNode::text_trim)
            && !platform.is_empty()
        {
            metadata.platform = Some(platform.to_string());
        }
        if let Some(region) = meta.child("Region").map(XmlNode::text_trim)
            && !region.is_empty()
        {
            metadata.region = Some(region.to_string());
        }
        // The `<Variables>` container was split into `<SpeedrunComVariables>`
        // + `<CustomVariables>` in LiveSplit 1.8 (confirmed against a real
        // 1.8-written file) — read all three that might be present, since a
        // file could in principle carry either shape depending on the
        // LiveSplit version that last wrote it.
        for container in ["Variables", "SpeedrunComVariables", "CustomVariables"] {
            if let Some(vars) = meta.child(container) {
                for var in vars.children_named("Variable") {
                    if let Some(name) = var.attr("name") {
                        metadata.variables.push(RunVariable {
                            name: name.to_string(),
                            value: var.text_trim().to_string(),
                        });
                    }
                }
            }
        }
        run.metadata = metadata;
    }

    if let Some(count) = root.child("AttemptCount") {
        run.attempts = count.text_trim().parse().unwrap_or(0);
    }

    if let Some(history) = root.child("AttemptHistory") {
        for attempt in history.children_named("Attempt") {
            let Some(run_index) = attempt.attr("id").and_then(|s| s.parse::<u32>().ok()) else {
                continue;
            };

            let real_time = attempt.child_time("RealTime");
            let game_time = attempt.child_time("GameTime");
            let ended = real_time.is_some() || game_time.is_some();
            let date = attempt
                .attr("ended")
                .or_else(|| attempt.attr("started"))
                .and_then(parse_dotnet_datetime);

            run.attempt_history.push(AttemptHistoryEntry {
                run_index,
                real_time,
                game_time,
                ended,
                date,
            });
        }
    }

    if let Some(segs) = root.child("Segments") {
        // Running cumulative total per comparison name, so we can convert
        // LiveSplit's cumulative SplitTimes into our relative ones.
        let mut cumulative: BTreeMap<String, (Duration, Duration)> = BTreeMap::new();

        for (i, seg) in segs.children_named("Segment").enumerate() {
            let Some(split) = run.splits.get_mut(i) else {
                break;
            };

            if let Some(icon_node) = seg.child("Icon") {
                let cdata = icon_node.text_trim();
                if !cdata.is_empty()
                    && let Some(icon_path) = extract_icon(cdata, icons_dir, i)
                {
                    split.icon_path = Some(icon_path);
                }
            }

            if let Some(split_times) = seg.child("SplitTimes") {
                for st in split_times.children_named("SplitTime") {
                    let Some(name) = st.attr("name") else {
                        continue;
                    };
                    let cum_real = st.child_time("RealTime");
                    let cum_game = st.child_time("GameTime");

                    let (prev_real, prev_game) = cumulative
                        .get(name)
                        .copied()
                        .unwrap_or((Duration::zero(), Duration::zero()));

                    let rel_real = cum_real.map(|c| c - prev_real);
                    let rel_game = cum_game.map(|c| c - prev_game);

                    let entry = split.comparisons.entry(name.to_string()).or_default();
                    entry.real_time = rel_real;
                    entry.game_time = rel_game;

                    cumulative.insert(
                        name.to_string(),
                        (cum_real.unwrap_or(prev_real), cum_game.unwrap_or(prev_game)),
                    );
                }
            }

            if let Some(best) = seg.child("BestSegmentTime") {
                let entry = split
                    .comparisons
                    .entry(COMPARISON_BEST_SEGMENTS.to_string())
                    .or_default();
                entry.real_time = best.child_time("RealTime");
                entry.game_time = best.child_time("GameTime");
            }

            if let Some(hist) = seg.child("SegmentHistory") {
                for time_node in hist.children_named("Time") {
                    let Some(run_index) = time_node.attr("id").and_then(|s| s.parse::<u32>().ok())
                    else {
                        continue;
                    };
                    let real_time = time_node.child_time("RealTime");
                    let game_time = time_node.child_time("GameTime");
                    if real_time.is_none() && game_time.is_none() {
                        continue; // self-closing <Time id="N" />: no data for that attempt
                    }
                    split.segment_history.push(SegmentHistoryEntry {
                        run_index,
                        real_time,
                        game_time,
                    });
                }
            }
        }
    }

    Ok(ImportResult {
        run,
        source_version,
    })
}

/// LiveSplit embeds icons as a `.NET BinaryFormatter`-serialized
/// `System.Drawing.Bitmap`, not a plain base64 PNG. Fully implementing that
/// binary format is out of scope for icon support, so instead this scans
/// the decoded bytes for an embedded PNG (signature through `IEND`), which
/// is how `Bitmap`-wrapped PNGs are actually stored — a common trick other
/// `.lss` tooling uses. Returns `None` (silently) if no PNG is found.
fn extract_icon(cdata_base64: &str, icons_dir: &Path, index: usize) -> Option<String> {
    let bytes = base64_decode(cdata_base64)?;

    const PNG_SIGNATURE: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    let start = bytes.windows(8).position(|w| w == PNG_SIGNATURE)?;
    let iend_offset = bytes[start..].windows(4).position(|w| w == b"IEND")?;
    let end = (start + iend_offset + 4 + 4).min(bytes.len()); // "IEND" + 4-byte CRC
    let png_bytes = &bytes[start..end];

    std::fs::create_dir_all(icons_dir).ok()?;
    let file_name = format!("imported_{index}.png");
    std::fs::write(icons_dir.join(&file_name), png_bytes).ok()?;
    Some(format!("icons/{file_name}"))
}

/// Minimal standard-alphabet base64 decoder (avoids pulling in a whole
/// crate just to unwrap one CDATA blob).
fn base64_decode(input: &str) -> Option<Vec<u8>> {
    fn value(c: u8) -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }

    let cleaned: Vec<u8> = input.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    let mut out = Vec::with_capacity(cleaned.len() / 4 * 3);

    for chunk in cleaned.chunks(4) {
        if chunk.len() < 2 {
            return None;
        }
        let pad = chunk.iter().filter(|&&b| b == b'=').count();
        let mut buf = [0u8; 4];
        for (i, &b) in chunk.iter().enumerate() {
            buf[i] = if b == b'=' { 0 } else { value(b)? };
        }
        let n = ((buf[0] as u32) << 18)
            | ((buf[1] as u32) << 12)
            | ((buf[2] as u32) << 6)
            | (buf[3] as u32);
        out.push((n >> 16) as u8);
        if pad < 2 {
            out.push((n >> 8) as u8);
        }
        if pad < 1 {
            out.push(n as u8);
        }
    }

    Some(out)
}

// ---------------------------------------------------------------------
// Export
// ---------------------------------------------------------------------

/// Exports a `Run` to a LiveSplit-compatible `.lss` file. Icons are never
/// embedded (see module docs) — `<Icon />` is always empty.
pub fn export(run: &Run, path: &Path) -> Result<(), String> {
    let xml = build_xml(run);
    std::fs::write(path, xml).map_err(|e| format!("Failed to write {}: {e}", path.display()))
}

fn build_xml(run: &Run) -> String {
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str("<Run version=\"1.7.0\">\n");
    out.push_str("  <GameIcon />\n");
    out.push_str(&format!(
        "  <GameName>{}</GameName>\n",
        xml_escape(&run.title)
    ));
    out.push_str(&format!(
        "  <CategoryName>{}</CategoryName>\n",
        xml_escape(&run.category)
    ));

    out.push_str("  <Metadata>\n");
    out.push_str(&format!(
        "    <Run id=\"{}\" />\n",
        xml_escape(
            run.metadata
                .speedrun_com_category_id
                .as_deref()
                .unwrap_or("")
        )
    ));
    out.push_str(&format!(
        "    <Platform usesEmulator=\"False\">{}</Platform>\n",
        xml_escape(run.metadata.platform.as_deref().unwrap_or(""))
    ));
    out.push_str(&format!(
        "    <Region>{}</Region>\n",
        xml_escape(run.metadata.region.as_deref().unwrap_or(""))
    ));
    out.push_str("    <Variables>\n");
    for var in &run.metadata.variables {
        out.push_str(&format!(
            "      <Variable name=\"{}\">{}</Variable>\n",
            xml_escape(&var.name),
            xml_escape(&var.value)
        ));
    }
    out.push_str("    </Variables>\n");
    out.push_str("  </Metadata>\n");

    let offset_seconds = -run.start_offset.unwrap_or(0);
    out.push_str(&format!(
        "  <Offset>{}</Offset>\n",
        format_dotnet_timespan(Duration::seconds(offset_seconds))
    ));
    out.push_str(&format!(
        "  <AttemptCount>{}</AttemptCount>\n",
        run.attempts
    ));

    out.push_str("  <AttemptHistory>\n");
    for attempt in &run.attempt_history {
        let date = attempt
            .date
            .map(|d| d.format("%m/%d/%Y %H:%M:%S").to_string())
            .unwrap_or_else(|| "01/01/2000 00:00:00".to_string());

        if attempt.ended && (attempt.real_time.is_some() || attempt.game_time.is_some()) {
            out.push_str(&format!(
                "    <Attempt id=\"{}\" started=\"{date}\" isStartedSynced=\"True\" ended=\"{date}\" isEndedSynced=\"True\">\n",
                attempt.run_index
            ));
            if let Some(rt) = attempt.real_time {
                out.push_str(&format!(
                    "      <RealTime>{}</RealTime>\n",
                    format_dotnet_timespan(rt)
                ));
            }
            if let Some(gt) = attempt.game_time {
                out.push_str(&format!(
                    "      <GameTime>{}</GameTime>\n",
                    format_dotnet_timespan(gt)
                ));
            }
            out.push_str("    </Attempt>\n");
        } else {
            out.push_str(&format!(
                "    <Attempt id=\"{}\" started=\"{date}\" isStartedSynced=\"True\" ended=\"{date}\" isEndedSynced=\"True\" />\n",
                attempt.run_index
            ));
        }
    }
    out.push_str("  </AttemptHistory>\n");

    out.push_str("  <Segments>\n");
    let mut cumulative: BTreeMap<String, (Duration, Duration)> = BTreeMap::new();

    for split in &run.splits {
        out.push_str("    <Segment>\n");
        out.push_str(&format!("      <Name>{}</Name>\n", xml_escape(&split.name)));
        out.push_str("      <Icon />\n");

        out.push_str("      <SplitTimes>\n");
        for (name, cmp) in &split.comparisons {
            // Best Segments is written as BestSegmentTime below (LiveSplit
            // convention); Average/Median are LiveSplit-computed from
            // SegmentHistory and never stored — we don't have those keys in
            // `comparisons` anyway (they're computed on the fly).
            if name.as_str() == COMPARISON_BEST_SEGMENTS {
                continue;
            }

            let (prev_real, prev_game) = cumulative
                .get(name)
                .copied()
                .unwrap_or((Duration::zero(), Duration::zero()));
            let cum_real = cmp.real_time.map(|r| prev_real + r);
            let cum_game = cmp.game_time.map(|g| prev_game + g);

            out.push_str(&format!(
                "        <SplitTime name=\"{}\">\n",
                xml_escape(name)
            ));
            if let Some(c) = cum_real {
                out.push_str(&format!(
                    "          <RealTime>{}</RealTime>\n",
                    format_dotnet_timespan(c)
                ));
            }
            if let Some(c) = cum_game {
                out.push_str(&format!(
                    "          <GameTime>{}</GameTime>\n",
                    format_dotnet_timespan(c)
                ));
            }
            out.push_str("        </SplitTime>\n");

            cumulative.insert(
                name.clone(),
                (cum_real.unwrap_or(prev_real), cum_game.unwrap_or(prev_game)),
            );
        }
        out.push_str("      </SplitTimes>\n");

        if let Some(best) = split.comparisons.get(COMPARISON_BEST_SEGMENTS) {
            out.push_str("      <BestSegmentTime>\n");
            if let Some(r) = best.real_time {
                out.push_str(&format!(
                    "        <RealTime>{}</RealTime>\n",
                    format_dotnet_timespan(r)
                ));
            }
            if let Some(g) = best.game_time {
                out.push_str(&format!(
                    "        <GameTime>{}</GameTime>\n",
                    format_dotnet_timespan(g)
                ));
            }
            out.push_str("      </BestSegmentTime>\n");
        }

        out.push_str("      <SegmentHistory>\n");
        for entry in &split.segment_history {
            if entry.real_time.is_none() && entry.game_time.is_none() {
                continue;
            }
            out.push_str(&format!("        <Time id=\"{}\">\n", entry.run_index));
            if let Some(r) = entry.real_time {
                out.push_str(&format!(
                    "          <RealTime>{}</RealTime>\n",
                    format_dotnet_timespan(r)
                ));
            }
            if let Some(g) = entry.game_time {
                out.push_str(&format!(
                    "          <GameTime>{}</GameTime>\n",
                    format_dotnet_timespan(g)
                ));
            }
            out.push_str("        </Time>\n");
        }
        out.push_str("      </SegmentHistory>\n");

        out.push_str("    </Segment>\n");
    }
    out.push_str("  </Segments>\n");
    out.push_str("</Run>\n");

    out
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
