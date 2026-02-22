use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

pub struct PreviewItem {
    pub scene_id: String,
    pub raw_rel: String,
    pub final_rel: String,
}

pub fn write_index(path: &Path, items: &[PreviewItem]) -> Result<()> {
    let mut cards = String::new();
    for item in items {
        cards.push_str(&format!(
            r#"<section class="card">
  <h2>{scene}</h2>
  <div class="grid">
    <figure><figcaption>Raw</figcaption><img src="{raw}" alt="raw {scene}" loading="lazy"/></figure>
    <figure><figcaption>Final</figcaption><img src="{final_img}" alt="final {scene}" loading="lazy"/></figure>
  </div>
</section>
"#,
            scene = html_escape(&item.scene_id),
            raw = html_escape(&item.raw_rel),
            final_img = html_escape(&item.final_rel)
        ));
    }

    let html = format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Screenforge Preview</title>
  <style>
    body {{
      margin: 0;
      padding: 24px;
      font-family: ui-sans-serif, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
      background: #0c111b;
      color: #e4ebf6;
    }}
    h1 {{ margin: 0 0 24px 0; font-size: 28px; }}
    .card {{
      margin-bottom: 28px;
      background: #11192a;
      border: 1px solid #263449;
      border-radius: 12px;
      padding: 16px;
    }}
    .card h2 {{
      margin: 0 0 12px 0;
      font-size: 16px;
      letter-spacing: 0.04em;
      text-transform: uppercase;
      color: #bfd0ea;
    }}
    .grid {{
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
      gap: 16px;
    }}
    figure {{
      margin: 0;
      background: #0c111b;
      border-radius: 10px;
      padding: 10px;
      border: 1px solid #213046;
    }}
    figcaption {{
      margin-bottom: 10px;
      font-size: 12px;
      opacity: 0.8;
      text-transform: uppercase;
      letter-spacing: 0.08em;
    }}
    img {{
      width: 100%;
      height: auto;
      border-radius: 8px;
      display: block;
      background: #070b13;
    }}
  </style>
</head>
<body>
  <h1>Screenforge Preview</h1>
  {cards}
</body>
</html>"#
    );

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed creating {}", parent.display()))?;
    }
    fs::write(path, html).with_context(|| format!("failed writing {}", path.display()))?;
    Ok(())
}

fn html_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
