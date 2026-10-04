//! A compiler graph derived only by guarded composition of admitted binary minis.

use sha2::{Digest, Sha256};

use crate::manager_rebuild::ManagerBinaryProviderV1;

use super::header::CacheHeader;
use super::remap::PristineNativeApiAuthority;
use super::selective_fullgraph::{SelectiveFullGraphChange, SelectiveFullGraphOutput};
use super::splice::{
    remap_module_to_base_with_loadout_plan, LoadoutScriptIdPlanBuilder, SequentialMiniGuard,
};
use super::walk_modules::module_names;

const MAX_CACHE_BYTES: usize = 512 * 1024 * 1024;
const MAX_PROVIDER_BYTES: u64 = 4 * 1024 * 1024 * 1024;
const MAX_PROVIDERS: usize = 100_000;

/// Fields are private: a caller cannot assert that arbitrary replacement bytes derive from
/// pristine. Native declaration authority remains the original cache's authority throughout.
pub(crate) struct ManagerBinaryGraphV1 {
    pristine_sha256: [u8; 32],
    binds_sha256: [u8; 32],
    cache: Vec<u8>,
    modules: Vec<String>,
    native_authority: PristineNativeApiAuthority,
}

impl ManagerBinaryGraphV1 {
    pub(crate) fn cache(&self) -> &[u8] {
        &self.cache
    }

    pub(crate) fn modules(&self) -> &[String] {
        &self.modules
    }

    pub(super) fn native_authority(&self) -> &PristineNativeApiAuthority {
        &self.native_authority
    }

    pub(crate) fn matches_pristine(&self, pristine: &[u8], binds: &[u8]) -> bool {
        self.pristine_sha256 == <[u8; 32]>::from(Sha256::digest(pristine))
            && self.binds_sha256 == <[u8; 32]>::from(Sha256::digest(binds))
            && self.native_authority.matches_pristine(pristine)
    }

    pub(crate) fn matches_compile_inputs(&self, cache: &[u8], binds: &[u8]) -> bool {
        self.cache == cache && self.binds_sha256 == <[u8; 32]>::from(Sha256::digest(binds))
    }

    pub(crate) fn compose_sources(
        &self,
        rebuilt: &[u8],
        changes: Vec<SelectiveFullGraphChange>,
    ) -> Result<SelectiveFullGraphOutput, super::selective_fullgraph::SelectiveFullGraphError> {
        super::selective_fullgraph::compose_manager_binary_graph_sources(self, rebuilt, changes)
    }
}

fn provider_identity(provider: &ManagerBinaryProviderV1) -> Result<[u8; 32], String> {
    if provider.mini_cache.is_empty() || provider.mini_cache.len() > MAX_CACHE_BYTES {
        return Err("binary provider mini is empty or exceeds 512 MiB".into());
    }
    if provider.op != "add" && provider.op != "edit" {
        return Err("binary provider operation must be add or edit".into());
    }
    if provider.module.is_empty()
        || provider.module.len() > 4096
        || provider.module.chars().any(char::is_control)
    {
        return Err("invalid binary provider root module".into());
    }
    let names = module_names(&provider.mini_cache).map_err(|error| error.to_string())?;
    if !names.contains(&provider.module) {
        return Err("binary provider root is not carried by its mini".into());
    }
    let mut hash = Sha256::new();
    hash.update(provider.op.as_bytes());
    hash.update([0]);
    hash.update(provider.module.as_bytes());
    hash.update([0, u8::from(provider.allow_edit_as_add)]);
    hash.update(Sha256::digest(&provider.mini_cache));
    Ok(hash.finalize().into())
}

/// The reader supplies one private, sealed mini at a time. A second pass verifies its identity
/// before canonicalization, so neither the memory envelope nor trust depends on retaining all
/// binary payloads at once. Each mini must independently pass existing pristine admission.
pub(crate) fn build_manager_binary_graph_v1<F>(
    pristine: &[u8],
    binds: &[u8],
    provider_count: usize,
    mut read_provider: F,
) -> Result<ManagerBinaryGraphV1, String>
where
    F: FnMut(usize) -> Result<ManagerBinaryProviderV1, String>,
{
    if provider_count == 0 || provider_count > MAX_PROVIDERS {
        return Err("invalid binary provider count".into());
    }
    if pristine.is_empty() || pristine.len() > MAX_CACHE_BYTES {
        return Err("invalid pristine binary graph cache size".into());
    }
    let mut inventory = LoadoutScriptIdPlanBuilder::new_with_binds(pristine, binds)
        .map_err(|error| error.to_string())?;
    let mut identities = Vec::with_capacity(provider_count);
    let mut total = 0u64;
    let mut modules = std::collections::BTreeSet::new();
    for index in 0..provider_count {
        let provider = read_provider(index)?;
        identities.push(provider_identity(&provider)?);
        total = total
            .checked_add(provider.mini_cache.len() as u64)
            .filter(|total| *total <= MAX_PROVIDER_BYTES)
            .ok_or("binary providers exceed the 4-GiB envelope")?;
        for name in module_names(&provider.mini_cache).map_err(|error| error.to_string())? {
            if !modules.insert(name) {
                return Err("binary providers contain overlapping module winners".into());
            }
        }
        inventory
            .inspect(&provider.mini_cache)
            .map_err(|error| error.to_string())?;
    }
    let plan = inventory.finish().map_err(|error| error.to_string())?;
    let native_authority = PristineNativeApiAuthority::from_pristine(pristine, binds);
    let mut guard =
        SequentialMiniGuard::new_with_native_authority(pristine, Some(&native_authority))
            .map_err(|error| error.to_string())?;
    let mut running = pristine.to_vec();
    for (index, expected) in identities.iter().enumerate() {
        let provider = read_provider(index)?;
        if provider_identity(&provider)? != *expected {
            return Err("binary provider changed between graph passes".into());
        }
        let mini = remap_module_to_base_with_loadout_plan(&provider.mini_cache, pristine, &plan)
            .map_err(|error| error.to_string())?;
        let carried = module_names(&mini).map_err(|error| error.to_string())?;
        running = match provider.op.as_str() {
            "add" => guard.compose_add(&running, &mini),
            "edit" if provider.allow_edit_as_add => guard.compose_upsert(&running, &mini),
            "edit" if carried.len() > 1 => {
                let names = module_names(&running).map_err(|error| error.to_string())?;
                if !carried.iter().any(|name| names.contains(name)) {
                    return Err(
                        "binary edit carries no existing module in the selected graph".into(),
                    );
                }
                guard.compose_upsert(&running, &mini)
            }
            "edit" => guard.compose_edit(&running, &mini, &provider.module),
            _ => unreachable!("validated operation"),
        }
        .map_err(|error| error.to_string())?;
        if running.len() > MAX_CACHE_BYTES {
            return Err("binary compiler graph exceeds 512 MiB".into());
        }
    }
    if CacheHeader::parse(pristine)
        .map_err(|error| error.to_string())?
        .hash
        != CacheHeader::parse(&running)
            .map_err(|error| error.to_string())?
            .hash
    {
        return Err("binary graph changed pristine generation".into());
    }
    Ok(ManagerBinaryGraphV1 {
        pristine_sha256: Sha256::digest(pristine).into(),
        binds_sha256: Sha256::digest(binds).into(),
        cache: running,
        modules: modules.into_iter().collect(),
        native_authority,
    })
}
