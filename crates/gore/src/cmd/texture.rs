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
    /// Replace a texture with a PNG, writing rewritten cooked files into a mod dir
    Replace {
        /// Path to the game install dir (contains G1R/Content/Paks/…)
        #[arg(long)]
        game: PathBuf,
        /// Cooked asset path, e.g. /Game/UI/Textures/Common/T_HardwareCursor
        asset: String,
        /// Replacement PNG (RGBA8 / RGB8); dims need not match the original
        #[arg(long)]
        image: PathBuf,
        /// Output mod dir; rewritten cooked files land under <mod_dir>/G1R/Content/…
        #[arg(long)]
        mod_dir: PathBuf,
    },
    /// Pack a mod dir of cooked files into a Zen triplet (.utoc/.ucas/.pak)
    Pack {
        /// Path to the game install dir (needed for the global script-objects store)
        #[arg(long)]
        game: PathBuf,
        /// Mod dir laid out under its mount path (from `texture replace`)
        #[arg(long)]
        mod_dir: PathBuf,
        /// Base name for the triplet, e.g. zzz_MyMod_P
        #[arg(long)]
        name: String,
        /// Output dir for <name>.{utoc,ucas,pak}
        #[arg(short = 'o', long)]
        out: PathBuf,
        /// Oodle-compress the .ucas blocks (opt-in). Default OFF: uncompressed
        /// containers are proven to load in-game; compressed ones are currently
        /// ignored by the game (unresolved Oodle framing issue).
        #[arg(long)]
        compress: bool,
    },
    /// Deploy a Zen triplet into the game's ~mods override folder
    Deploy {
        /// Path to the game install dir (contains G1R/Content/Paks/…)
        #[arg(long)]
        game: PathBuf,
        /// Dir holding <name>.{utoc,ucas,pak} (the `texture pack` output dir)
        #[arg(long)]
        triplet_dir: PathBuf,
        /// Base name of the triplet, e.g. zzz_MyMod_P
        #[arg(long)]
        name: String,
    },
    /// Remove a previously-deployed triplet from the game's ~mods folder
    Undeploy {
        /// Path to the game install dir (contains G1R/Content/Paks/…)
        #[arg(long)]
        game: PathBuf,
        /// Base name of the deployed triplet, e.g. zzz_MyMod_P
        #[arg(long)]
        name: String,
    },
}

/// Compute the cooked mount path for an asset under a mod dir, per the gore-tex
/// layout rule: `<mod_dir>/G1R/Content/` + (asset path with a leading `/Game/`
/// replaced by `` and any other leading `/` stripped). Returns the directory the
/// `<leaf>.{uasset,uexp,ubulk}` files should be written into.
fn mount_dir(mod_dir: &std::path::Path, asset: &str) -> PathBuf {
    let rel = if let Some(stripped) = asset.strip_prefix("/Game/") {
        stripped.to_string()
    } else {
        asset.trim_start_matches('/').to_string()
    };
    // Drop the leaf (file stem); we only want the containing dir under Content/.
    let dir_rel = rel.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
    mod_dir.join("G1R/Content").join(dir_rel)
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
        TextureAction::Replace {
            game,
            asset,
            image: image_path,
            mod_dir,
        } => {
            let utoc = gore_tex::paths::main_container(&game)?;
            let usmap = gore_tex::paths::usmap(&game)?;

            // 1. Unpack the original cooked files + learn its format/dims.
            let tmp = std::env::temp_dir().join("gore-tex-replace");
            let _ = std::fs::remove_dir_all(&tmp);
            std::fs::create_dir_all(&tmp)
                .with_context(|| format!("creating temp dir {}", tmp.display()))?;

            let uasset = gore_tex::container::unpack_asset(&utoc, &usmap, &asset, &tmp)
                .with_context(|| format!("unpacking asset {asset}"))?;
            let uexp = uasset.with_extension("uexp");
            let ubulk = uasset.with_extension("ubulk");

            let orig_uasset =
                std::fs::read(&uasset).with_context(|| format!("reading {}", uasset.display()))?;
            let orig_uexp =
                std::fs::read(&uexp).with_context(|| format!("reading {}", uexp.display()))?;
            let orig_ubulk = std::fs::read(&ubulk).unwrap_or_default();

            let info = gore_tex::decode::parse(&orig_uasset, &orig_uexp, &orig_ubulk, &std::fs::read(&usmap)?)
                .with_context(|| format!("decoding original texture {asset}"))?;
            let format = info.format.clone();

            // 2. Load the replacement PNG -> RGBA8 bytes + dims.
            let img = image::open(&image_path)
                .with_context(|| format!("opening {}", image_path.display()))?
                .to_rgba8();
            let (w, h) = img.dimensions();
            let rgba = img.into_raw();

            // 3. Encode mips in the original pixel format, then rewrite the cooked files.
            let mips = gore_tex::encode::encode_mips(&rgba, w, h, &format)
                .with_context(|| format!("encoding mips ({w}x{h} {format})"))?;
            let (new_uasset, new_uexp, new_ubulk) = gore_tex::texdata::replace_texture(
                &orig_uasset,
                &orig_uexp,
                &orig_ubulk,
                w,
                h,
                mips,
            )
            .with_context(|| format!("rewriting cooked texture {asset}"))?;

            // 4. Write the rewritten triplet under the asset's mount path in mod_dir.
            let dir = mount_dir(&mod_dir, &asset);
            std::fs::create_dir_all(&dir)
                .with_context(|| format!("creating {}", dir.display()))?;
            let leaf = asset.rsplit('/').next().unwrap_or(&asset);
            let out_uasset = dir.join(format!("{leaf}.uasset"));
            let out_uexp = dir.join(format!("{leaf}.uexp"));
            std::fs::write(&out_uasset, &new_uasset)
                .with_context(|| format!("writing {}", out_uasset.display()))?;
            std::fs::write(&out_uexp, &new_uexp)
                .with_context(|| format!("writing {}", out_uexp.display()))?;
            if !new_ubulk.is_empty() {
                let out_ubulk = dir.join(format!("{leaf}.ubulk"));
                std::fs::write(&out_ubulk, &new_ubulk)
                    .with_context(|| format!("writing {}", out_ubulk.display()))?;
            }

            println!(
                "wrote {} ({}x{} {}, was {}x{}){}",
                out_uasset.display(),
                w,
                h,
                format,
                info.width,
                info.height,
                if new_ubulk.is_empty() {
                    " [inline]".to_string()
                } else {
                    format!(" [+{} bytes streamed]", new_ubulk.len())
                },
            );
            Ok(())
        }
        TextureAction::Pack {
            game,
            mod_dir,
            name,
            out,
            compress,
        } => {
            let triplet = gore_tex::container::repack_to_zen(&mod_dir, &name, &out, &game, compress)
                .with_context(|| format!("packing {} into {name}", mod_dir.display()))?;
            println!("wrote triplet:");
            for p in &triplet {
                println!("  {}", p.display());
            }
            Ok(())
        }
        TextureAction::Deploy {
            game,
            triplet_dir,
            name,
        } => {
            let triplet = [
                triplet_dir.join(format!("{name}.utoc")),
                triplet_dir.join(format!("{name}.ucas")),
                triplet_dir.join(format!("{name}.pak")),
            ];
            for p in &triplet {
                if !p.exists() {
                    anyhow::bail!("triplet file missing: {}", p.display());
                }
            }
            let record = gore_tex::container::deploy(&triplet, &game, &name)
                .with_context(|| format!("deploying {name}"))?;
            println!("deployed to ~mods; record: {}", record.display());
            println!("launch the game to see it.");
            Ok(())
        }
        TextureAction::Undeploy { game, name } => {
            gore_tex::container::undeploy(&game, &name)
                .with_context(|| format!("undeploying {name}"))?;
            println!("undeployed {name} (removed from ~mods).");
            Ok(())
        }
    }
}
