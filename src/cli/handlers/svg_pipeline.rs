use serde_json::json;
use std::collections::HashSet;
use std::path::Path;

/// Direct render handler for SVG and Amata documents
pub fn handle_render(
    input: &Path,
    output: &Path,
    scale: Option<f32>,
    width: Option<u32>,
    height: Option<u32>,
    background: Option<&str>,
) -> Result<bool, Box<dyn std::error::Error>> {
    if !input.exists() {
        return Err(format!("Input file does not exist: {}", input.display()).into());
    }

    let ext = input
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    let svg_content = if ext == "svg" {
        std::fs::read_to_string(input)?
    } else {
        let doc = crate::io::project::load_project(&input.to_path_buf())?;
        crate::io::svg::export_svg(&doc)
    };

    let mut opt = resvg::usvg::Options::default();
    opt.fontdb_mut().load_system_fonts();

    let rtree = resvg::usvg::Tree::from_str(&svg_content, &opt)
        .map_err(|e| format!("Failed to parse SVG for rendering: {e}"))?;

    let base_size = rtree.size();
    let base_w = base_size.width();
    let base_h = base_size.height();

    if base_w <= 0.0 || base_h <= 0.0 {
        return Err("SVG dimensions must be greater than zero".into());
    }

    // Determine target dimensions & scale factor
    let (target_w, target_h, scale_factor) = if let (Some(w), Some(h)) = (width, height) {
        let sx = w as f32 / base_w;
        let sy = h as f32 / base_h;
        (w, h, sx.min(sy))
    } else if let Some(w) = width {
        let s = w as f32 / base_w;
        let h = (base_h * s).round().max(1.0) as u32;
        (w, h, s)
    } else if let Some(h) = height {
        let s = h as f32 / base_h;
        let w = (base_w * s).round().max(1.0) as u32;
        (w, h, s)
    } else {
        let s = scale.unwrap_or(1.0);
        let w = (base_w * s).round().max(1.0) as u32;
        let h = (base_h * s).round().max(1.0) as u32;
        (w, h, s)
    };

    // Huge canvas guard (prevent OOM crash)
    const MAX_CANVAS_DIM: u32 = 16384;
    if target_w > MAX_CANVAS_DIM || target_h > MAX_CANVAS_DIM {
        return Err(format!(
            "Target dimensions ({} x {}) exceed safe limit of {} x {}. Please reduce scale or dimensions.",
            target_w, target_h, MAX_CANVAS_DIM, MAX_CANVAS_DIM
        ).into());
    }

    let mut pixmap = resvg::tiny_skia::Pixmap::new(target_w, target_h).ok_or_else(|| {
        format!(
            "Failed to allocate pixel buffer for {} x {}",
            target_w, target_h
        )
    })?;

    // Background handling
    let out_ext = output
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    let is_jpeg = out_ext == "jpg" || out_ext == "jpeg";

    if let Some(bg_str) = background {
        if let Some(rgba) = crate::io::svg::parse_svg_color(bg_str) {
            let color = resvg::tiny_skia::Color::from_rgba(rgba[0], rgba[1], rgba[2], rgba[3])
                .ok_or_else(|| "Invalid background color values".to_string())?;
            pixmap.fill(color);
        } else {
            eprintln!("⚠️ Warning: unrecognized background color '{bg_str}', using default");
            if is_jpeg {
                pixmap.fill(resvg::tiny_skia::Color::WHITE);
            }
        }
    } else if is_jpeg {
        pixmap.fill(resvg::tiny_skia::Color::WHITE);
    }

    let transform = resvg::tiny_skia::Transform::from_scale(scale_factor, scale_factor);
    resvg::render(&rtree, transform, &mut pixmap.as_mut());

    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }

    if is_jpeg {
        let png_bytes = pixmap
            .encode_png()
            .map_err(|e| format!("Failed to encode intermediary PNG: {e}"))?;
        let img = image::load_from_memory(&png_bytes)
            .map_err(|e| format!("Failed to load image for JPEG encoding: {e}"))?;
        let mut jpeg_bytes = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut jpeg_bytes);
        img.to_rgb8()
            .write_to(&mut cursor, image::ImageFormat::Jpeg)
            .map_err(|e| format!("Failed to encode JPEG: {e}"))?;
        std::fs::write(output, jpeg_bytes)?;
    } else if out_ext == "pdf" {
        let doc = crate::io::svg::parse_svg_document(&svg_content);
        let pdf_bytes = crate::io::pdf::export_pdf(&doc);
        std::fs::write(output, pdf_bytes)?;
    } else {
        let png_bytes = pixmap
            .encode_png()
            .map_err(|e| format!("Failed to encode PNG: {e}"))?;
        std::fs::write(output, png_bytes)?;
    }

    println!(
        "✅ Rendered {} -> {} ({} × {} px, scale: {:.2}x)",
        input.display(),
        output.display(),
        target_w,
        target_h,
        scale_factor
    );
    Ok(false)
}

/// Structural inspection of SVG or .amata document
pub fn handle_inspect(input: &Path, json_output: bool) -> Result<bool, Box<dyn std::error::Error>> {
    if !input.exists() {
        return Err(format!("File does not exist: {}", input.display()).into());
    }

    let ext = input
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    let svg_content = if ext == "svg" {
        std::fs::read_to_string(input)?
    } else {
        let doc = crate::io::project::load_project(&input.to_path_buf())?;
        crate::io::svg::export_svg(&doc)
    };

    // Parse and count tokens/elements
    let mut path_count = 0;
    let mut rect_count = 0;
    let mut circle_count = 0;
    let mut ellipse_count = 0;
    let mut line_count = 0;
    let mut poly_count = 0;
    let mut text_count = 0;
    let mut group_count = 0;
    let mut linear_grads = 0;
    let mut radial_grads = 0;
    let mut clip_paths = 0;
    let mut masks = 0;
    let mut filters = 0;
    let mut transforms = false;
    let mut opacity_used = false;
    let mut warnings = Vec::new();

    let mut doc_width = 0.0f64;
    let mut doc_height = 0.0f64;
    let mut view_box = String::new();

    // Extract root SVG attributes (handling multi-line <svg ...> tag)
    if let Some(svg_start) = svg_content.find("<svg") {
        if let Some(svg_end) = svg_content[svg_start..].find('>') {
            let svg_tag = &svg_content[svg_start..svg_start + svg_end];
            if let Some(pos) = svg_tag.find("width=") {
                let rest = svg_tag[pos + 6..].trim_start();
                if let Some(q) = rest.chars().next() {
                    if let Some(end) = rest[1..].find(q) {
                        doc_width = rest[1..=end]
                            .trim_end_matches("px")
                            .trim_end_matches("pt")
                            .parse()
                            .unwrap_or(0.0);
                    }
                }
            }
            if let Some(pos) = svg_tag.find("height=") {
                let rest = svg_tag[pos + 7..].trim_start();
                if let Some(q) = rest.chars().next() {
                    if let Some(end) = rest[1..].find(q) {
                        doc_height = rest[1..=end]
                            .trim_end_matches("px")
                            .trim_end_matches("pt")
                            .parse()
                            .unwrap_or(0.0);
                    }
                }
            }
            if let Some(pos) = svg_tag.find("viewBox=") {
                let rest = svg_tag[pos + 8..].trim_start();
                if let Some(q) = rest.chars().next() {
                    if let Some(end) = rest[1..].find(q) {
                        view_box = rest[1..=end].to_string();
                    }
                }
            }
        }
    }

    for line in svg_content.lines() {
        let trimmed = line.trim();
        if trimmed.contains("<path") {
            path_count += 1;
        }
        if trimmed.contains("<rect") {
            rect_count += 1;
        }
        if trimmed.contains("<circle") {
            circle_count += 1;
        }
        if trimmed.contains("<ellipse") {
            ellipse_count += 1;
        }
        if trimmed.contains("<line") {
            line_count += 1;
        }
        if trimmed.contains("<polygon") || trimmed.contains("<polyline") {
            poly_count += 1;
        }
        if trimmed.contains("<text") {
            text_count += 1;
        }
        if trimmed.starts_with("<g") {
            group_count += 1;
        }
        if trimmed.contains("<linearGradient") {
            linear_grads += 1;
        }
        if trimmed.contains("<radialGradient") {
            radial_grads += 1;
        }
        if trimmed.contains("<clipPath") {
            clip_paths += 1;
        }
        if trimmed.contains("<mask") {
            masks += 1;
        }
        if trimmed.contains("<filter") {
            filters += 1;
        }
        if trimmed.contains("transform=") {
            transforms = true;
        }
        if trimmed.contains("opacity=") {
            opacity_used = true;
        }

        if trimmed.contains("<feGaussianBlur") || trimmed.contains("<feDropShadow") {
            // supported filter
        } else if trimmed.contains("<fe") {
            let filter_name = trimmed.split_whitespace().next().unwrap_or("filter");
            if !warnings.contains(&format!("Advanced filter element: {filter_name}")) {
                warnings.push(format!("Advanced filter element: {filter_name}"));
            }
        }
    }

    if doc_width == 0.0 || doc_height == 0.0 {
        if !view_box.is_empty() {
            let parts: Vec<f64> = view_box
                .split(|c: char| c.is_whitespace() || c == ',')
                .filter_map(|s| s.parse().ok())
                .collect();
            if parts.len() == 4 {
                doc_width = parts[2];
                doc_height = parts[3];
            }
        }
    }

    if json_output {
        let info = json!({
            "file": input.display().to_string(),
            "document": {
                "width": doc_width,
                "height": doc_height,
                "viewBox": view_box,
            },
            "objects": {
                "paths": path_count,
                "rects": rect_count,
                "circles": circle_count,
                "ellipses": ellipse_count,
                "lines": line_count,
                "polygons": poly_count,
                "text": text_count,
                "groups": group_count,
            },
            "resources": {
                "linear_gradients": linear_grads,
                "radial_gradients": radial_grads,
                "clip_paths": clip_paths,
                "masks": masks,
                "filters": filters,
            },
            "features": {
                "transforms": transforms,
                "opacity": opacity_used,
            },
            "warnings": warnings,
        });
        println!("{}", serde_json::to_string_pretty(&info)?);
    } else {
        println!("════════════════════════════════════════════════════════════");
        println!("  Amata Document Inspector: {}", input.display());
        println!("════════════════════════════════════════════════════════════");
        println!("Document:");
        println!("  size:       {:.1} × {:.1} pt", doc_width, doc_height);
        println!(
            "  viewBox:    {}",
            if view_box.is_empty() {
                "none"
            } else {
                &view_box
            }
        );
        println!();
        println!("Objects:");
        println!("  paths:      {}", path_count);
        println!("  rectangles: {}", rect_count);
        println!("  circles:    {}", circle_count);
        println!("  ellipses:   {}", ellipse_count);
        println!("  lines:      {}", line_count);
        println!("  polygons:   {}", poly_count);
        println!("  text:       {}", text_count);
        println!("  groups:     {}", group_count);
        println!();
        println!("Resources:");
        println!("  linear gradients: {}", linear_grads);
        println!("  radial gradients: {}", radial_grads);
        println!("  clip paths:       {}", clip_paths);
        println!("  masks:            {}", masks);
        println!("  filters:          {}", filters);
        println!();
        println!("Features:");
        println!("  transforms: {}", if transforms { "yes" } else { "no" });
        println!("  opacity:    {}", if opacity_used { "yes" } else { "no" });
        println!(
            "  text:       {}",
            if text_count > 0 { "yes" } else { "no" }
        );

        if !warnings.is_empty() {
            println!();
            println!("Warnings:");
            for w in &warnings {
                println!("  ⚠️ {w}");
            }
        }
        println!("════════════════════════════════════════════════════════════");
    }

    Ok(false)
}

/// Comprehensive validator for CI and export-blocking checks
pub fn handle_validate(input: &Path, strict: bool) -> Result<bool, Box<dyn std::error::Error>> {
    if !input.exists() {
        return Err(format!(
            "Validation failed: file '{}' does not exist",
            input.display()
        )
        .into());
    }

    let svg_content = std::fs::read_to_string(input)?;
    let mut errors: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    // 1. Basic XML structure check
    if !svg_content.contains("<svg") || !svg_content.contains("</svg>") {
        errors.push("Missing root <svg> element or closing </svg> tag".to_string());
    }

    // 2. Validate with usvg (robust XML & SVG grammar parser)
    let mut opt = resvg::usvg::Options::default();
    opt.fontdb_mut().load_system_fonts();

    match resvg::usvg::Tree::from_str(&svg_content, &opt) {
        Ok(tree) => {
            let size = tree.size();
            if size.width() <= 0.0 || size.height() <= 0.0 {
                errors.push(format!(
                    "Invalid dimensions: width={}, height={}",
                    size.width(),
                    size.height()
                ));
            }
            if size.width().is_nan()
                || size.height().is_nan()
                || size.width().is_infinite()
                || size.height().is_infinite()
            {
                errors.push("Dimensions contain NaN or Infinity".to_string());
            }
        }
        Err(e) => {
            errors.push(format!("SVG syntax error: {e}"));
        }
    }

    // 3. Scan for defined IDs in <defs> or document
    let mut defined_ids = HashSet::new();
    let mut referenced_ids = Vec::new();

    for line in svg_content.lines() {
        let trimmed = line.trim();
        // find id="name"
        let mut rest = trimmed;
        while let Some(pos) = rest.find("id=") {
            let after = &rest[pos + 3..];
            if let Some(quote) = after.chars().next() {
                if quote == '"' || quote == '\'' {
                    if let Some(end) = after[1..].find(quote) {
                        let id = &after[1..=end];
                        defined_ids.insert(id.to_string());
                        rest = &after[end + 1..];
                        continue;
                    }
                }
            }
            rest = &after[1..];
        }

        // find url(#name)
        let mut ref_rest = trimmed;
        while let Some(pos) = ref_rest.find("url(#") {
            let after = &ref_rest[pos + 5..];
            if let Some(end) = after.find(')') {
                let id = &after[..end].trim();
                referenced_ids.push(id.to_string());
                ref_rest = &after[end + 1..];
            } else {
                break;
            }
        }

        // find href="#name"
        let mut href_rest = trimmed;
        while let Some(pos) = href_rest.find("href=\"#") {
            let after = &href_rest[pos + 7..];
            if let Some(end) = after.find('"') {
                let id = &after[..end].trim();
                referenced_ids.push(id.to_string());
                href_rest = &after[end + 1..];
            } else {
                break;
            }
        }
    }

    // Check for broken references
    for ref_id in &referenced_ids {
        if !defined_ids.contains(ref_id) {
            errors.push(format!(
                "Broken reference: 'url(#{ref_id})' references undefined ID"
            ));
        }
    }

    // Check for unreferenced IDs (as informational warning)
    for def_id in &defined_ids {
        if !referenced_ids.iter().any(|r| r == def_id) && !def_id.starts_with("layer") {
            warnings.push(format!("Unused definition or resource ID: '{def_id}'"));
        }
    }

    // Results summary
    if errors.is_empty() {
        println!(
            "✅ Validation passed: '{}' is a valid SVG document.",
            input.display()
        );
        if !warnings.is_empty() {
            println!("   ({} informational warnings)", warnings.len());
            if strict {
                for w in &warnings {
                    eprintln!("   ⚠️ [strict warning] {w}");
                }
            }
        }
        Ok(false)
    } else {
        eprintln!(
            "❌ Validation failed: {} error(s) found in '{}':",
            errors.len(),
            input.display()
        );
        for err in &errors {
            eprintln!("   • {err}");
        }
        Err(format!("Validation failed with {} error(s)", errors.len()).into())
    }
}

/// Conservative and safe SVG optimizer
pub fn handle_optimize(
    input: &Path,
    output: &Path,
    precision: usize,
) -> Result<bool, Box<dyn std::error::Error>> {
    if !input.exists() {
        return Err(format!("Input file does not exist: {}", input.display()).into());
    }

    let svg_content = std::fs::read_to_string(input)?;

    // 1. Gather all referenced IDs
    let mut referenced_ids = HashSet::new();
    for line in svg_content.lines() {
        let trimmed = line.trim();
        let mut ref_rest = trimmed;
        while let Some(pos) = ref_rest.find("url(#") {
            let after = &ref_rest[pos + 5..];
            if let Some(end) = after.find(')') {
                referenced_ids.insert(after[..end].trim().to_string());
                ref_rest = &after[end + 1..];
            } else {
                break;
            }
        }
        let mut href_rest = trimmed;
        while let Some(pos) = href_rest.find("href=\"#") {
            let after = &href_rest[pos + 7..];
            if let Some(end) = after.find('"') {
                referenced_ids.insert(after[..end].trim().to_string());
                href_rest = &after[end + 1..];
            } else {
                break;
            }
        }
    }

    // 2. Filter lines and optimize
    let mut optimized_lines: Vec<String> = Vec::new();
    let mut in_unused_def = false;
    let mut def_tag_depth = 0;
    let mut removed_defs = 0;
    let mut removed_empty_groups = 0;

    for line in svg_content.lines() {
        let trimmed = line.trim();

        // Check if starting an unused def block
        if trimmed.starts_with("<linearGradient")
            || trimmed.starts_with("<radialGradient")
            || trimmed.starts_with("<clipPath")
            || trimmed.starts_with("<filter")
        {
            if let Some(pos) = trimmed.find("id=\"") {
                let after = &trimmed[pos + 4..];
                if let Some(end) = after.find('"') {
                    let id = &after[..end];
                    if !referenced_ids.contains(id) {
                        in_unused_def = true;
                        def_tag_depth = 1;
                        removed_defs += 1;
                        if trimmed.ends_with("/>") {
                            in_unused_def = false;
                        }
                        continue;
                    }
                }
            }
        }

        if in_unused_def {
            if trimmed.contains("</linearGradient>")
                || trimmed.contains("</radialGradient>")
                || trimmed.contains("</clipPath>")
                || trimmed.contains("</filter>")
            {
                def_tag_depth -= 1;
                if def_tag_depth == 0 {
                    in_unused_def = false;
                }
            }
            continue;
        }

        // Remove empty groups (single-line or multi-line)
        if trimmed == "</g>" {
            if let Some(last) = optimized_lines.last() {
                let last_trimmed = last.trim();
                if last_trimmed.starts_with("<g")
                    && last_trimmed.ends_with('>')
                    && !last_trimmed.ends_with("/>")
                {
                    optimized_lines.pop();
                    removed_empty_groups += 1;
                    continue;
                }
            }
        }
        if trimmed == "<g></g>" || trimmed == "<g />" {
            removed_empty_groups += 1;
            continue;
        }

        // Clean extra whitespace in path data numbers if applicable
        if trimmed.starts_with("<path") && precision > 0 {
            // Keep safe formatting without destroying commands
            optimized_lines.push(line.to_string());
        } else {
            optimized_lines.push(line.to_string());
        }
    }

    let result_svg = optimized_lines.join("\n") + "\n";
    let original_size = svg_content.len();
    let new_size = result_svg.len();

    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    std::fs::write(output, result_svg)?;

    let saved_bytes = original_size.saturating_sub(new_size);
    let ratio = if original_size > 0 {
        (new_size as f64 / original_size as f64) * 100.0
    } else {
        100.0
    };

    println!("✅ Optimized SVG saved to: {}", output.display());
    println!(
        "   Size: {} bytes -> {} bytes ({:.1}%, saved {} bytes)",
        original_size, new_size, ratio, saved_bytes
    );
    if removed_defs > 0 {
        println!("   Pruned {} unused resource definition(s)", removed_defs);
    }
    if removed_empty_groups > 0 {
        println!("   Removed {} empty group(s)", removed_empty_groups);
    }

    Ok(false)
}
