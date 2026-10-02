//! A standalone report keeps visual Editor information available in the CLI.
use super::*;
use base64::Engine;
fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
fn label(key: &str) -> String {
    super::text::readable(key)
}
fn scalar(value: &Value) -> String {
    match value {
        Value::Null => "—".into(),
        Value::String(s) => escape(s),
        _ => escape(&value.to_string()),
    }
}
fn render(value: &Value, depth: usize, show_ids: bool) -> String {
    match value {
        Value::Array(rows) => {
            if rows.is_empty() {
                return "<span class=muted>—</span>".into();
            }
            if rows.iter().all(Value::is_object) && rows.len() < 6000 {
                let mut keys = std::collections::BTreeSet::new();
                for row in rows {
                    keys.extend(
                        row.as_object()
                            .unwrap()
                            .keys()
                            .filter(|k| {
                                *k != "bytesBase64"
                                    && (show_ids
                                        || !matches!(
                                            k.as_str(),
                                            "id" | "globalId"
                                                | "uniqueName"
                                                | "questClass"
                                                | "documentClass"
                                                | "statePath"
                                                | "setPath"
                                        ))
                            })
                            .cloned(),
                    );
                }
                let keys = keys.into_iter().collect::<Vec<_>>();
                format!("<div class=scroll><table><thead><tr>{}</tr></thead><tbody>{}</tbody></table></div>",keys.iter().map(|k|format!("<th>{}</th>",escape(&label(k)))).collect::<String>(),rows.iter().map(|row|format!("<tr>{}</tr>",keys.iter().map(|k|format!("<td title=\"{}\">{}</td>",escape(row["tooltip"].as_str().or(row["presentation"]["tooltip"].as_str()).unwrap_or("")),render(&row[k],depth+1,show_ids))).collect::<String>())).collect::<String>())
            } else {
                format!(
                    "<ul>{}</ul>",
                    rows.iter()
                        .map(|r| format!("<li>{}</li>", render(r, depth + 1, show_ids)))
                        .collect::<String>()
                )
            }
        }
        Value::Object(map) => format!(
            "<dl>{}</dl>",
            map.iter()
                .filter(|(k, _)| k.as_str() != "bytesBase64")
                .map(|(k, v)| format!(
                    "<dt>{}</dt><dd>{}</dd>",
                    escape(&label(k)),
                    if v.is_object() || v.is_array() {
                        format!(
                            "<details{}><summary>{}</summary>{}</details>",
                            if depth < 1 { " open" } else { "" },
                            escape(&label(k)),
                            render(v, depth + 1, show_ids)
                        )
                    } else {
                        scalar(v)
                    }
                ))
                .collect::<String>()
        ),
        _ => scalar(value),
    }
}
fn data_image(path: &Path) -> Result<String> {
    let metadata = fs::metadata(path)?;
    if !metadata.is_file() || metadata.len() > 32 * 1024 * 1024 {
        bail!("report image exceeds its size limit");
    }
    let bytes = fs::read(path)?;
    if image::guess_format(&bytes)? != image::ImageFormat::Png {
        bail!("report artwork must be PNG");
    }
    Ok(format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}
pub(super) fn html(data: &Value, settings: &Value, o: &Options) -> Result<String> {
    let texts = super::text::Texts::load(&o.lang)?;
    let title = texts.ui("appTitle");
    let title = if title == "appTitle" {
        "GORE Save report".into()
    } else {
        title
    };
    let title_for = |key: &str| {
        let ui = match key {
            "attributes" => "tabAttribute",
            "inventory" => "tabInventory",
            "characters" => "tabCharacters",
            "quests" => "sectionQuests",
            "knowledge" => "sectionKnowledge",
            "events" => "sectionEvents",
            "glossary" => "sectionGlossary",
            "traders" => "tabTrade",
            "worldTime" => "tabWorld",
            "inspection" => "tabOverview",
            _ => "",
        };
        if ui.is_empty() {
            label(key)
        } else {
            texts.ui(ui)
        }
    };
    let dark = settings["themeMode"] == "dark";
    let scale = settings["uiScale"].as_f64().unwrap_or(1.0).clamp(0.5, 2.0);
    let (font, bytes): (&str, Option<&[u8]>) = match (
        settings["uiFontFamily"].as_str().unwrap_or_else(|| {
            if settings.get("gothicUiFont").is_some() {
                if settings["gothicUiFont"] == true {
                    "podkova"
                } else {
                    "system"
                }
            } else {
                "notoSerif"
            }
        }),
        o.lang.as_str(),
    ) {
        ("system", _) => ("system-ui", None),
        (_, "ja") => (
            "Editor",
            Some(include_bytes!(
                "../../../../../apps/save-editor/assets/fonts/NotoSerifJP-Variable.ttf"
            )),
        ),
        (_, "zh-Hans") => (
            "Editor",
            Some(include_bytes!(
                "../../../../../apps/save-editor/assets/fonts/NotoSerifSC-Variable.ttf"
            )),
        ),
        ("podkova", _) => (
            "Editor",
            Some(include_bytes!(
                "../../../../../apps/save-editor/assets/fonts/Podkova-Variable.ttf"
            )),
        ),
        _ => (
            "Editor",
            Some(include_bytes!(
                "../../../../../apps/save-editor/assets/fonts/NotoSerif-Variable.ttf"
            )),
        ),
    };
    let mut font_css=bytes.map(|bytes|format!("@font-face{{font-family:Editor;src:url(data:font/ttf;base64,{}) format('truetype');font-weight:100 900}}",base64::engine::general_purpose::STANDARD.encode(bytes))).unwrap_or_default();
    if settings["themeMode"] == "system" {
        font_css.push_str("@media(prefers-color-scheme:dark){body{background:#171b21!important;color:#eee!important}:root{color-scheme:dark!important}}");
    }
    let show_ids = o.show_ids || settings["showObjectIds"] == true;
    let mut body = format!("<h1>{}</h1>", escape(&title));
    let screenshot = &data["inspection"]["screenshot"];
    if let (Some(mime), Some(bytes)) = (
        screenshot["mimeType"].as_str(),
        screenshot["bytesBase64"].as_str(),
    ) {
        if matches!(mime, "image/png" | "image/jpeg" | "image/webp") {
            body.push_str(&format!(
                "<figure><img alt=\"Save screenshot\" src=\"data:{mime};base64,{}\"></figure>",
                escape(bytes)
            ));
        }
    }
    let order = [
        "statistics",
        "inspection",
        "worldTime",
        "attributes",
        "skills",
        "inventory",
        "characters",
        "quests",
        "tutorials",
        "glossary",
        "story",
        "knowledge",
        "events",
        "factions",
        "traders",
    ];
    body.push_str("<nav>");
    for key in order {
        body.push_str(&format!(
            "<a href=\"#{key}\">{}</a> ",
            escape(&title_for(key))
        ));
    }
    body.push_str("</nav>");
    for key in order {
        if let Some(value) = data.get(key) {
            body.push_str(&format!(
                "<section id=\"{key}\"><h2>{}</h2>{}</section>",
                escape(&title_for(key)),
                render(value, 0, show_ids)
            ));
        }
    }
    if o.with_assets {
        body.push_str("<section><h2>Artwork and symbols</h2><div class=gallery>");
        if let Some(rows) = data["artwork"]["entries"].as_array() {
            let root = data["artwork"]["sourceRoot"]
                .as_str()
                .map(Path::new)
                .and_then(|p| p.canonicalize().ok());
            for row in rows
                .iter()
                .filter(|r| r["available"] == true && r["size"] == "M")
            {
                let path = Path::new(row["path"].as_str().context("artwork path missing")?)
                    .canonicalize()?;
                if root.as_ref().is_none_or(|root| !path.starts_with(root)) {
                    bail!("artwork escapes its source");
                }
                body.push_str(&format!("<figure><img loading=lazy src=\"{}\" alt=\"{}\"><figcaption>{}</figcaption></figure>",data_image(&path)?,escape(row["name"].as_str().unwrap_or("")),escape(row["name"].as_str().unwrap_or(""))));
            }
        }
        if let Some(manifest) = data["icons"]["manifestPath"].as_str() {
            let manifest = Path::new(manifest);
            let verified = gore_tex::item_icons::verified_item_icon_manifest(manifest)?;
            let root = manifest
                .parent()
                .context("manifest parent missing")?
                .canonicalize()?;
            for (id, relative) in verified.items {
                let path = root.join(relative).canonicalize()?;
                if !path.starts_with(&root) {
                    bail!("icon escapes cache");
                }
                body.push_str(&format!("<figure><img class=icon loading=lazy src=\"{}\" alt=\"{}\"><figcaption>{}</figcaption></figure>",data_image(&path)?,escape(&id),escape(&id)));
            }
        }
        body.push_str("</div></section>");
    }
    body.push_str(&format!(
        "<details><summary>Diagnostic JSON</summary><pre>{}</pre></details>",
        escape(&serde_json::to_string_pretty(data)?)
    ));
    Ok(format!("<!doctype html><html lang=\"{}\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>{}</title><style>{font_css}:root{{color-scheme:{}}}body{{background:{};color:{};font:{}px '{font}',serif;margin:2rem}}nav{{display:flex;flex-wrap:wrap;gap:1rem}}section{{margin-block:2rem}}.scroll{{overflow:auto}}table{{border-collapse:collapse}}th,td{{padding:.5rem;border:1px solid #8885;text-align:left;vertical-align:top}}dl{{display:grid;grid-template-columns:minmax(8rem,auto) 1fr;gap:.5rem}}dd{{margin:0;min-width:0}}img{{max-width:100%;max-height:500px}}.gallery{{display:flex;flex-wrap:wrap;gap:1rem}}.gallery figure{{margin:0;max-width:280px}}.icon{{max-width:96px;max-height:96px}}pre{{white-space:pre-wrap;overflow-wrap:anywhere}}a{{color:inherit}}</style></head><body>{body}</body></html>",escape(&o.lang),escape(&title),if dark{"dark"}else{"light"},if dark{"#171b21"}else{"#faf8f2"},if dark{"#eee"}else{"#232323"},16.0*scale))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn report_escapes_game_text_and_preserves_tooltips_and_preferences() {
        let options = Options {
            lang: "de".into(),
            ..Default::default()
        };
        let html = html(
            &json!({"attributes":[{"label":"<script>x</script>","tooltip":"a\"b","base":4}]}),
            &json!({"themeMode":"dark","uiScale":1.5,"uiFontFamily":"system"}),
            &options,
        )
        .unwrap();
        assert!(!html.contains("<script>x"));
        assert!(html.contains("&lt;script&gt;"));
        assert!(html.contains("a&quot;b"));
        assert!(html.contains("font:24px"));
        assert!(html.contains("lang=\"de\""));
        assert!(html.contains("color-scheme:dark"));
    }
}
