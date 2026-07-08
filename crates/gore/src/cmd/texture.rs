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
        game: Option<PathBuf>,
        /// Keep only asset paths containing this substring
        #[arg(long)]
        filter: Option<String>,
    },
    /// Extract a texture's top mip to a PNG
    Extract {
        /// Path to the game install dir (contains G1R/Content/Paks/…)
        #[arg(long)]
        game: Option<PathBuf>,
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
        game: Option<PathBuf>,
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
        game: Option<PathBuf>,
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
        /// containers are proven to load in-game. The compressed path follows
        /// the base game's writer conventions (raw ContainerHeader, 1 KiB
        /// admission threshold, 16-aligned blocks).
        #[arg(long)]
        compress: bool,
    },
    /// Deploy a Zen triplet into the game's ~mods override folder
    Deploy {
        /// Path to the game install dir (contains G1R/Content/Paks/…)
        #[arg(long)]
        game: Option<PathBuf>,
        /// Dir holding <name>.{utoc,ucas,pak} (the `texture pack` output dir)
        #[arg(long)]
        triplet_dir: PathBuf,
        /// Base name of the triplet, e.g. zzz_MyMod_P
        #[arg(long)]
        name: String,
    },
    /// Build the texture index (asset->package_id) and cache it to the shared dir
    Index {
        #[arg(long)]
        game: Option<PathBuf>,
        /// Output path (defaults to the shared gore-tools texture_index.json)
        #[arg(short = 'o', long)]
        out: Option<PathBuf>,
    },
    /// Remove a previously-deployed triplet from the game's ~mods folder
    Undeploy {
        /// Path to the game install dir (contains G1R/Content/Paks/…)
        #[arg(long)]
        game: Option<PathBuf>,
        /// Base name of the deployed triplet, e.g. zzz_MyMod_P
        #[arg(long)]
        name: String,
    },
}

/// Compute the cooked mount path for an asset under a mod dir, per the gore-tex
/// layout rule: `<mod_dir>/G1R/Content/` + (asset path with a leading `/Game/`
/// replaced by `` and any other leading `/` stripped). Returns the directory the
/// `<leaf>.{uasset,uexp,ubulk}` files should be written into.
fn mount_dir(mod_dir: &std::path::Path, asset: &str) -> Result<PathBuf> {
    // Map the UE mount root to its physical content path (/Game -> G1R/Content,
    // /Engine -> Engine/Content). Forcing a non-/Game asset under G1R/Content
    // would place the override at the wrong virtual path so it never applies;
    // unknown roots (plugins) are rejected.
    let rel = gore_tex::paths::content_mount_rel(asset).ok_or_else(|| {
        anyhow::anyhow!("unsupported asset mount root (only /Game and /Engine): {asset}")
    })?;
    // Drop the leaf (file stem); we only want the containing dir.
    let dir_rel = rel.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
    Ok(mod_dir.join(dir_rel))
}

/// Reject a triplet `--name` that isn't a single filename component. Without
/// this, a value like `../foo` or `a/b` flows into `format!("{name}.utoc")` joins
/// (pack/deploy) and would write or delete files OUTSIDE the intended directory.
fn validate_triplet_name(name: &str) -> Result<()> {
    use std::path::Component;
    let mut comps = std::path::Path::new(name).components();
    match (comps.next(), comps.next()) {
        (Some(Component::Normal(_)), None) => Ok(()),
        _ => anyhow::bail!(
            "invalid --name {name:?}: must be a single filename with no path separators or '..'"
        ),
    }
}

pub fn run(action: TextureAction) -> Result<()> {
    match action {
        TextureAction::List { game, filter } => {
            let game = gore_loc::config::game_root(game)?;
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
            let game = gore_loc::config::game_root(game)?;
            let utoc = gore_tex::paths::main_container(&game)?;
            let usmap = gore_tex::paths::usmap(&game)?;

            let tmp = gore_tex::paths::unique_temp_dir("gore-tex-extract")
                .with_context(|| "creating temp dir")?;

            let uasset = gore_tex::container::unpack_asset(&utoc, &usmap, &asset, &tmp)
                .with_context(|| format!("unpacking asset {asset}"))?;
            let uexp = uasset.with_extension("uexp");
            let ubulk = uasset.with_extension("ubulk");

            let info = gore_tex::decode::parse(
                &std::fs::read(&uasset)
                    .with_context(|| format!("reading {}", uasset.display()))?,
                &std::fs::read(&uexp)
                    .with_context(|| format!("reading {}", uexp.display()))?,
                &gore_tex::paths::read_optional(&ubulk)
                    .with_context(|| format!("reading {}", ubulk.display()))?,
                &std::fs::read(&usmap)
                    .with_context(|| format!("reading {}", usmap.display()))?,
            )
            .with_context(|| format!("decoding texture {asset}"))?;

            // Cooked files are now in `info`; drop the unpack temp dir so repeated
            // extracts don't accumulate large .uasset/.uexp/.ubulk payloads in temp.
            let _ = std::fs::remove_dir_all(&tmp);

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
            let game = gore_loc::config::game_root(game)?;
            let utoc = gore_tex::paths::main_container(&game)?;
            let usmap = gore_tex::paths::usmap(&game)?;

            // 1. Unpack the original cooked files + learn its format/dims. Use a
            //    per-call unique temp dir so concurrent `texture replace` runs don't
            //    share (and delete) each other's unpacked cooked files.
            let tmp = gore_tex::paths::unique_temp_dir("gore-tex-replace")
                .with_context(|| "creating temp dir")?;

            let uasset = gore_tex::container::unpack_asset(&utoc, &usmap, &asset, &tmp)
                .with_context(|| format!("unpacking asset {asset}"))?;
            let uexp = uasset.with_extension("uexp");
            let ubulk = uasset.with_extension("ubulk");

            let orig_uasset =
                std::fs::read(&uasset).with_context(|| format!("reading {}", uasset.display()))?;
            let orig_uexp =
                std::fs::read(&uexp).with_context(|| format!("reading {}", uexp.display()))?;
            let orig_ubulk = gore_tex::paths::read_optional(&ubulk)
                .with_context(|| format!("reading {}", ubulk.display()))?;

            // Originals are now in memory; drop the unpack temp dir so repeated
            // replaces don't accumulate cooked payloads in the system temp dir.
            let _ = std::fs::remove_dir_all(&tmp);

            let info = gore_tex::decode::parse(&orig_uasset, &orig_uexp, &orig_ubulk, &std::fs::read(&usmap)?)
                .with_context(|| format!("decoding original texture {asset}"))?;
            let format = info.format.clone();

            // 2. Load the replacement PNG -> RGBA8 bytes + dims.
            let img = image::open(&image_path)
                .with_context(|| format!("opening {}", image_path.display()))?
                .to_rgba8();
            let (w, h) = img.dimensions();
            let rgba = img.into_raw();

            // 3. Rewrite the cooked files. The unified entry encodes mips (regular
            //    texture) or re-tiles (virtual texture) internally based on the
            //    original's shape, so we always pass the raw RGBA + format.
            let (new_uasset, new_uexp, new_ubulk) = gore_tex::texdata::replace_texture_image(
                &orig_uasset,
                &orig_uexp,
                &orig_ubulk,
                &rgba,
                w,
                h,
                &format,
            )
            .with_context(|| format!("rewriting cooked texture {asset}"))?;

            // 4. Write the rewritten triplet under the asset's mount path in mod_dir.
            let dir = mount_dir(&mod_dir, &asset)?;
            std::fs::create_dir_all(&dir)
                .with_context(|| format!("creating {}", dir.display()))?;
            let leaf = asset.rsplit('/').next().unwrap_or(&asset);
            let out_uasset = dir.join(format!("{leaf}.uasset"));
            let out_uexp = dir.join(format!("{leaf}.uexp"));
            std::fs::write(&out_uasset, &new_uasset)
                .with_context(|| format!("writing {}", out_uasset.display()))?;
            std::fs::write(&out_uexp, &new_uexp)
                .with_context(|| format!("writing {}", out_uexp.display()))?;
            let out_ubulk = dir.join(format!("{leaf}.ubulk"));
            if !new_ubulk.is_empty() {
                std::fs::write(&out_ubulk, &new_ubulk)
                    .with_context(|| format!("writing {}", out_ubulk.display()))?;
            } else {
                // New rewrite is inline (no streamed bulk). Remove any stale
                // `.ubulk` left by a prior replacement into this same mod dir,
                // else `texture pack` would pair it with the new .uasset/.uexp.
                let _ = std::fs::remove_file(&out_ubulk);
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
            let game = gore_loc::config::game_root(game)?;
            validate_triplet_name(&name)?;
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
            let game = gore_loc::config::game_root(game)?;
            validate_triplet_name(&name)?;
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
        TextureAction::Index { game, out } => {
            let game = gore_loc::config::game_root(game)?;
            let utoc = gore_tex::paths::main_container(&game)?;
            let usmap = gore_tex::paths::usmap(&game)?;
            let build_id = gore_tex::index::build_id_for(&utoc, &usmap);
            eprintln!("scanning container to build the texture index (a few minutes)...");
            let idx = gore_tex::index::build_index(&utoc, &build_id)?;
            let path = out.unwrap_or_else(gore_tex::paths::texture_index_path);
            idx.save(&path)?;
            println!("wrote {} ({} textures)", path.display(), idx.entries.len());
            Ok(())
        }
        TextureAction::Undeploy { game, name } => {
            let game = gore_loc::config::game_root(game)?;
            validate_triplet_name(&name)?;
            gore_tex::container::undeploy(&game, &name)
                .with_context(|| format!("undeploying {name}"))?;
            println!("undeployed {name} (removed from ~mods).");
            Ok(())
        }
    }
}
