//! `gore texture` — list and extract game textures from the UE5 IoStore
//! container (Gothic 1 Remake). Backed by the `gore-tex` crate.

use anyhow::{Context, Result};
use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum TextureAction {
    /// List Texture2D assets in the game container
    List {
        /// Path to the game install dir (contains G1R/Content/Paks/…)
        #[arg(long)]
        game: PathBuf,
        /// Keep only asset paths containing this substring
        #[arg(long)]
        filter: Option<String>,
    },
    /// Extract a texture's top mip to a PNG
    Extract {
        /// Path to the game install dir (contains G1R/Content/Paks/…)
        #[arg(long)]
        game: PathBuf,
        /// Cooked asset path, e.g. /Game/UI/Textures/Common/T_HardwareCursor
        asset: String,
        /// Output PNG path
        #[arg(short = 'o', long)]
        out: PathBuf,
    },
}

pub fn run(action: TextureAction) -> Result<()> {
    match action {
        TextureAction::List { game, filter } => {
            let utoc = gore_tex::paths::main_container(&game)?;
            let usmap = gore_tex::paths::usmap(&game)?;
            eprintln!(
                "scanning the whole container for textures; this can take several minutes..."
            );
            for e in gore_tex::container::list_textures(&utoc, &usmap, filter.as_deref())? {
                println!("{}", e.asset_path);
            }
            Ok(())
        }
        TextureAction::Extract { game, asset, out } => {
            let utoc = gore_tex::paths::main_container(&game)?;
            let usmap = gore_tex::paths::usmap(&game)?;

            let tmp = std::env::temp_dir().join("gore-tex-extract");
            std::fs::create_dir_all(&tmp)
                .with_context(|| format!("creating temp dir {}", tmp.display()))?;

            let uasset = gore_tex::container::unpack_asset(&utoc, &usmap, &asset, &tmp)
                .with_context(|| format!("unpacking asset {asset}"))?;
            let uexp = uasset.with_extension("uexp");
            let ubulk = uasset.with_extension("ubulk");

            let info = gore_tex::decode::parse(
                &std::fs::read(&uasset)
                    .with_context(|| format!("reading {}", uasset.display()))?,
                &std::fs::read(&uexp)
                    .with_context(|| format!("reading {}", uexp.display()))?,
                &std::fs::read(&ubulk).unwrap_or_default(),
                &std::fs::read(&usmap)
                    .with_context(|| format!("reading {}", usmap.display()))?,
            )
            .with_context(|| format!("decoding texture {asset}"))?;

            let px = gore_tex::decode::to_rgba8(&info)
                .with_context(|| format!("rgba decode of {asset}"))?;

            // to_rgba8 returns 0xAARRGGBB pixels; pack to [R, G, B, A] byte order.
            let mut buf = Vec::with_capacity(px.len() * 4);
            for p in px {
                buf.extend_from_slice(&[
                    (p >> 16) as u8,
                    (p >> 8) as u8,
                    p as u8,
                    (p >> 24) as u8,
                ]);
            }

            image::save_buffer(&out, &buf, info.width, info.height, image::ColorType::Rgba8)
                .with_context(|| format!("writing {}", out.display()))?;

            let sidecar = out.with_extension("png.json");
            std::fs::write(
                &sidecar,
                format!(
                    "{{\"asset\":\"{}\",\"width\":{},\"height\":{},\"format\":\"{}\"}}",
                    asset, info.width, info.height, info.format
                ),
            )
            .with_context(|| format!("writing {}", sidecar.display()))?;

            println!(
                "wrote {} ({}x{} {})",
                out.display(),
                info.width,
                info.height,
                info.format
            );
            Ok(())
        }
    }
}
