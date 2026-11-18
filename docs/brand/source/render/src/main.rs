use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use resvg::tiny_skia::{self, PathSegment};
use resvg::usvg;

const USAGE: &str = "usage:
  brand-render png <svg> <out.png> <width> [--bg #rrggbb]
  brand-render textpath <font.ttf> <family> <size> <letter-spacing> <text>";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let outcome = match args.first().map(String::as_str) {
        Some("png") => png(&args[1..]),
        Some("textpath") => textpath(&args[1..]),
        _ => Err(USAGE.to_owned()),
    };
    match outcome {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn options(fonts: &[PathBuf]) -> usvg::Options<'static> {
    let mut options = usvg::Options::default();
    let database = options.fontdb_mut();
    for font in fonts {
        database.load_font_file(font).ok();
    }
    options
}

fn assets_fonts() -> Vec<PathBuf> {
    let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../crates/app/assets");
    vec![
        assets.join("Mitr-SemiBold.ttf"),
        assets.join("Anuphan[wght].ttf"),
    ]
}

fn png(args: &[String]) -> Result<(), String> {
    let [svg, out, width, rest @ ..] = args else {
        return Err(USAGE.to_owned());
    };
    let width: u32 = width.parse().map_err(|_| "bad width".to_owned())?;
    let source = std::fs::read_to_string(svg).map_err(|error| format!("{svg}: {error}"))?;
    let tree = usvg::Tree::from_str(&source, &options(&assets_fonts()))
        .map_err(|error| format!("{svg}: {error}"))?;
    let size = tree.size();
    let scale = width as f32 / size.width();
    let height = (size.height() * scale).round() as u32;
    let mut pixmap = tiny_skia::Pixmap::new(width, height).ok_or("empty pixmap")?;
    if let [flag, color] = rest
        && flag == "--bg"
    {
        let hex = u32::from_str_radix(color.trim_start_matches('#'), 16)
            .map_err(|_| "bad colour".to_owned())?;
        pixmap.fill(tiny_skia::Color::from_rgba8(
            (hex >> 16) as u8,
            (hex >> 8) as u8,
            hex as u8,
            255,
        ));
    }
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    pixmap.save_png(out).map_err(|error| format!("{out}: {error}"))
}

fn textpath(args: &[String]) -> Result<(), String> {
    let [font, family, size, spacing, text] = args else {
        return Err(USAGE.to_owned());
    };
    let svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"2000\" height=\"600\"><text x=\"0\" y=\"300\" font-family=\"{family}\" font-size=\"{size}\" letter-spacing=\"{spacing}\" fill=\"#000\">{text}</text></svg>"
    );
    let tree = usvg::Tree::from_str(&svg, &options(&[PathBuf::from(font)]))
        .map_err(|error| error.to_string())?;
    let mut d = String::new();
    let mut bounds: Option<(f32, f32, f32, f32)> = None;
    collect(tree.root(), &mut d, &mut bounds);
    let (left, top, right, bottom) = bounds.ok_or("no glyphs")?;
    println!("{left:.2} {top:.2} {:.2} {:.2}", right - left, bottom - top);
    println!("{d}");
    Ok(())
}

fn collect(group: &usvg::Group, d: &mut String, bounds: &mut Option<(f32, f32, f32, f32)>) {
    for node in group.children() {
        match node {
            usvg::Node::Group(inner) => collect(inner, d, bounds),
            usvg::Node::Text(text) => collect(text.flattened(), d, bounds),
            usvg::Node::Path(path) => {
                let transform = path.abs_transform();
                let mut points = Vec::new();
                for segment in path.data().segments() {
                    match segment {
                        PathSegment::MoveTo(p) => points.push(('M', vec![p])),
                        PathSegment::LineTo(p) => points.push(('L', vec![p])),
                        PathSegment::QuadTo(a, b) => points.push(('Q', vec![a, b])),
                        PathSegment::CubicTo(a, b, c) => points.push(('C', vec![a, b, c])),
                        PathSegment::Close => points.push(('Z', vec![])),
                    }
                }
                for (command, mut list) in points {
                    for point in &mut list {
                        transform.map_point(point);
                        let (x, y) = (point.x, point.y);
                        *bounds = Some(match *bounds {
                            None => (x, y, x, y),
                            Some((l, t, r, b)) => (l.min(x), t.min(y), r.max(x), b.max(y)),
                        });
                    }
                    d.push(command);
                    for (index, point) in list.iter().enumerate() {
                        if index > 0 {
                            d.push(' ');
                        }
                        write!(d, "{:.2} {:.2}", point.x, point.y).ok();
                    }
                }
            }
            usvg::Node::Image(_) => {}
        }
    }
}
