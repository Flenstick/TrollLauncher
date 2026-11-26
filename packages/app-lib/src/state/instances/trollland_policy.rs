use super::adapters::sqlite::instance_rows;
use crate::state::{Instance, InstanceLink, State};
use serde_json::Value;
use sqlx::SqlitePool;
use std::fs::{self, File};
use std::io::{self, Cursor, Read, Seek};
use std::path::{Path, PathBuf};
use zip::ZipArchive;

const METADATA_LIMIT: u64 = 1024 * 1024;
const NESTED_JAR_LIMIT: u64 = 32 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ProhibitedContent {
	Freecam,
	Xray,
}

impl ProhibitedContent {
	pub(crate) fn name(self) -> &'static str {
		match self {
			Self::Freecam => "FreeCam",
			Self::Xray => "X-ray",
		}
	}

	pub(crate) fn error(self) -> crate::Error {
		crate::ErrorKind::InputError(format!(
			"{} запрещён в сборке TrollLand",
			self.name(),
		))
		.into()
	}
}

pub fn is_trollland_name(value: &str) -> bool {
	let value = value.trim().to_ascii_lowercase();
	value.strip_prefix("trollland").is_some_and(|suffix| {
		suffix.is_empty() || suffix.starts_with([' ', '-', '_', '.'])
	})
}

pub fn curseforge_block_reason(project_id: u64) -> Option<&'static str> {
	match project_id {
		557076 => Some("FreeCam запрещён в сборке TrollLand"),
		226375 | 256256 | 444663 => Some("X-ray запрещён в сборке TrollLand"),
		_ => None,
	}
}

pub async fn check_curseforge_download(
	instance_id: &str,
	project_id: u64,
	url: Option<&str>,
) -> crate::Result<()> {
	let reason = curseforge_block_reason(project_id)
		.map(crate::state::content_store::input)
		.or_else(|| {
			url.and_then(prohibited_download)
				.map(ProhibitedContent::error)
		});
	if let Some(error) = reason {
		let state = State::get().await?;
		if applies_to_id(instance_id, &state).await? {
			return Err(error);
		}
	}
	Ok(())
}

pub(crate) async fn applies_to(
	instance: &Instance,
	pool: &SqlitePool,
) -> crate::Result<bool> {
	if is_trollland_name(&instance.name) || is_trollland_name(&instance.path) {
		return Ok(true);
	}
	Ok(matches!(
		instance_rows::get_instance_link(&instance.id, pool).await?,
		InstanceLink::ImportedModpack { name, filename, .. }
			if name.iter().chain(filename.iter()).any(|name| is_trollland_name(name))
	))
}

pub(crate) async fn applies_to_id(
	instance_id: &str,
	state: &State,
) -> crate::Result<bool> {
	let instance = instance_rows::get_instance_by_id(instance_id, &state.pool)
		.await?
		.ok_or_else(|| {
			crate::state::content_store::input("Unknown instance")
		})?;
	applies_to(&instance, &state.pool).await
}

pub(crate) fn prohibited_project(id: &str) -> Option<ProhibitedContent> {
	match id {
		"XeEZ3fK2" | "freecam" => Some(ProhibitedContent::Freecam),
		"HbXXzLHU"
		| "pHENEouD"
		| "u6RWzf7M"
		| "m3F2KCrH"
		| "x-to-xray"
		| "xray-orevision"
		| "x-ray"
		| "xray-glow-spectator-x-ray" => Some(ProhibitedContent::Xray),
		_ => None,
	}
}

pub(crate) fn prohibited_download(url: &str) -> Option<ProhibitedContent> {
	let url = url::Url::parse(url).ok()?;
	if url.host_str() == Some("cdn.modrinth.com")
		&& let Some(project_id) = url
			.path()
			.strip_prefix("/data/")
			.and_then(|path| path.split('/').next())
		&& let Some(prohibited) = prohibited_project(project_id)
	{
		return Some(prohibited);
	}
	let file = url.path_segments()?.next_back()?.to_ascii_lowercase();
	match file.as_str() {
		name if name == "freecam.jar"
			|| (name.starts_with("freecam-") && name.ends_with(".jar")) =>
		{
			Some(ProhibitedContent::Freecam)
		}
		name if [
			"xray_ultimate_",
			"xray-ultimate-",
			"advanced-xray-",
			"advancedxray-",
			"xtoxray-",
		]
		.iter()
		.any(|prefix| name.starts_with(prefix)) =>
		{
			Some(ProhibitedContent::Xray)
		}
		_ => None,
	}
}

pub(crate) fn is_checked_path(path: &str) -> bool {
	path.split_once('/').is_some_and(|(folder, name)| {
		!name.is_empty()
			&& !name.contains('/')
			&& (folder.eq_ignore_ascii_case("mods")
				|| folder.eq_ignore_ascii_case("resourcepacks"))
	})
}

fn is_resource_pack(path: &str) -> bool {
	path.split_once('/')
		.is_some_and(|(folder, _)| folder.eq_ignore_ascii_case("resourcepacks"))
}

pub(crate) async fn inspect_path(
	path: &Path,
	relative: &str,
) -> crate::Result<Option<ProhibitedContent>> {
	if !is_checked_path(relative) {
		return Ok(None);
	}
	let path = path.to_owned();
	let is_pack = is_resource_pack(relative);
	tokio::task::spawn_blocking(move || inspect_file(&path, is_pack)).await?
}

pub(crate) fn inspect_bytes(
	bytes: &[u8],
	relative: &str,
) -> Option<ProhibitedContent> {
	is_checked_path(relative).then(|| {
		inspect_archive(Cursor::new(bytes), is_resource_pack(relative))
	})?
}

fn inspect_file(
	path: &Path,
	is_pack: bool,
) -> crate::Result<Option<ProhibitedContent>> {
	let metadata = match fs::metadata(path) {
		Ok(metadata) => metadata,
		Err(error) if error.kind() == io::ErrorKind::NotFound => {
			return Ok(None);
		}
		Err(error) => return Err(error.into()),
	};
	if is_pack && metadata.is_dir() {
		let meta = read_json_file(&path.join("pack.mcmeta"));
		return Ok(inspect_pack(meta, |name| path.join(name).is_file()));
	}
	if !metadata.is_file() {
		return Ok(None);
	}
	match File::open(path) {
		Ok(file) => Ok(inspect_archive(file, is_pack)),
		Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
		Err(error) => Err(error.into()),
	}
}

fn inspect_archive(
	reader: impl Read + Seek,
	is_pack: bool,
) -> Option<ProhibitedContent> {
	let mut archive = ZipArchive::new(reader).ok()?;
	if is_pack {
		let meta = read_json(&mut archive, "pack.mcmeta");
		return inspect_pack(meta, |name| {
			archive.index_for_name(name).is_some()
		});
	}
	inspect_mod(&mut archive).or_else(|| inspect_nested_mods(&mut archive))
}

fn prohibited_mod_id(id: &str) -> Option<ProhibitedContent> {
	match id {
		"freecam" => Some(ProhibitedContent::Freecam),
		"xray" | "advancedxray" | "advanced_xray" | "xtoxray" | "orevision"
		| "xray_orevision" => Some(ProhibitedContent::Xray),
		_ => None,
	}
}

fn inspect_mod(
	archive: &mut ZipArchive<impl Read + Seek>,
) -> Option<ProhibitedContent> {
	let fabric = read_json(archive, "fabric.mod.json");
	let quilt = read_json(archive, "quilt.mod.json");
	let json_id = fabric
		.as_ref()
		.and_then(|metadata| metadata["id"].as_str())
		.and_then(prohibited_mod_id)
		.or_else(|| {
			quilt
				.as_ref()?
				.pointer("/quilt_loader/id")?
				.as_str()
				.and_then(prohibited_mod_id)
		});
	json_id
		.or_else(|| {
			["META-INF/neoforge.mods.toml", "META-INF/mods.toml"]
				.into_iter()
				.find_map(|path| {
					let bytes = read_entry(archive, path, METADATA_LIMIT)?;
					let metadata: toml::Value =
						toml::from_str(std::str::from_utf8(&bytes).ok()?)
							.ok()?;
					metadata.get("mods")?.as_array()?.iter().find_map(|entry| {
						prohibited_mod_id(entry.get("modId")?.as_str()?)
					})
				})
		})
		.or_else(|| {
			archive
				.file_names()
				.filter(|name| name.ends_with(".class"))
				.find_map(|name| {
					if name.starts_with("net/xolt/freecam/") {
						Some(ProhibitedContent::Freecam)
					} else if name.starts_with("pro/mikey/xray/")
						|| name.starts_with("com/xtoxray/")
					{
						Some(ProhibitedContent::Xray)
					} else {
						None
					}
				})
		})
}

fn inspect_nested_mods(
	archive: &mut ZipArchive<impl Read + Seek>,
) -> Option<ProhibitedContent> {
	let fabric = read_json(archive, "fabric.mod.json");
	let forge = read_json(archive, "META-INF/jarjar/metadata.json");
	[fabric.as_ref(), forge.as_ref()]
		.into_iter()
		.flatten()
		.filter_map(|meta| meta["jars"].as_array())
		.flatten()
		.filter_map(|entry| {
			entry["file"].as_str().or_else(|| entry["path"].as_str())
		})
		.find_map(|path| {
			let bytes = read_entry(archive, path, NESTED_JAR_LIMIT)?;
			let mut nested = ZipArchive::new(Cursor::new(bytes)).ok()?;
			inspect_mod(&mut nested)
		})
}

fn inspect_pack(
	meta: Option<Value>,
	has_file: impl Fn(&str) -> bool,
) -> Option<ProhibitedContent> {
	let meta = meta?;
	let description = meta.pointer("/pack/description")?;
	let mut strings = Vec::new();
	collect_text(description, &mut strings);
	let description = strings.join(" ").to_ascii_lowercase();
	let ultimate_models = ["stone", "deepslate", "netherrack"]
		.into_iter()
		.filter(|block| {
			has_file(&format!(
				"assets/minecraft/models/block/xray/{block}.json"
			))
		})
		.count()
		>= 2;
	(ultimate_models
		|| description.contains("xray in spectator mode")
		|| (description.contains("xray glow")
			&& description.contains("netherspark studios")))
	.then_some(ProhibitedContent::Xray)
}

pub(crate) fn inspect_pack_metadata(
	bytes: &[u8],
	has_file: impl Fn(&str) -> bool,
) -> bool {
	inspect_pack(serde_json::from_slice(bytes).ok(), has_file).is_some()
}

fn collect_text<'a>(value: &'a Value, strings: &mut Vec<&'a str>) {
	match value {
		Value::String(text) => strings.push(text),
		Value::Array(children) => children
			.iter()
			.for_each(|child| collect_text(child, strings)),
		Value::Object(_) => {
			collect_text(&value["text"], strings);
			collect_text(&value["extra"], strings);
		}
		_ => {}
	}
}

fn read_json_file(path: &Path) -> Option<Value> {
	let mut reader = File::open(path).ok()?.take(METADATA_LIMIT + 1);
	let mut bytes = Vec::new();
	reader.read_to_end(&mut bytes).ok()?;
	if bytes.len() as u64 > METADATA_LIMIT {
		return None;
	}
	serde_json::from_slice(&bytes).ok()
}

fn read_json(
	archive: &mut ZipArchive<impl Read + Seek>,
	name: &str,
) -> Option<Value> {
	let bytes = read_entry(archive, name, METADATA_LIMIT)?;
	serde_json::from_slice(&bytes).ok()
}

fn read_entry(
	archive: &mut ZipArchive<impl Read + Seek>,
	name: &str,
	limit: u64,
) -> Option<Vec<u8>> {
	let entry = archive.by_name(name).ok()?;
	if entry.size() > limit {
		return None;
	}
	let mut bytes = Vec::new();
	entry.take(limit + 1).read_to_end(&mut bytes).ok()?;
	(bytes.len() as u64 <= limit).then_some(bytes)
}

pub(crate) struct ProhibitedFile {
	pub(crate) relative: String,
	pub(crate) content: ProhibitedContent,
	pub(crate) is_directory: bool,
}

pub(crate) async fn scan_instance(
	instance: &Instance,
	state: &State,
) -> crate::Result<Vec<ProhibitedFile>> {
	if !applies_to(instance, &state.pool).await? {
		return Ok(Vec::new());
	}
	let mods = state
		.content_store
		.instance_path(&instance.path, "mods/.policy-check")
		.await?;
	let packs = state
		.content_store
		.instance_path(&instance.path, "resourcepacks/.policy-check")
		.await?;
	tokio::task::spawn_blocking(move || scan_folders(mods, packs)).await?
}

pub(crate) async fn scan_import(
	root: &Path,
) -> crate::Result<Vec<ProhibitedFile>> {
	let mods = root.join("mods/.policy-check");
	let packs = root.join("resourcepacks/.policy-check");
	tokio::task::spawn_blocking(move || scan_folders(mods, packs)).await?
}

pub(crate) async fn unlink_file(path: &Path) -> crate::Result<()> {
	match tokio::fs::symlink_metadata(path).await {
		Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
		Err(error) => return Err(error.into()),
		Ok(_metadata) => {
			#[cfg(windows)]
			{
				use std::os::windows::fs::FileTypeExt;
				if _metadata.file_type().is_symlink_dir() {
					return Ok(tokio::fs::remove_dir(path).await?);
				}
			}
		}
	}
	crate::state::content_store::remove_instance_file(path).await
}

pub(crate) async fn remove_pack_directory(path: &Path) -> crate::Result<()> {
	match tokio::fs::remove_dir_all(path).await {
		Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
		result => Ok(result?),
	}
}

fn scan_folders(
	mod_probe: PathBuf,
	pack_probe: PathBuf,
) -> crate::Result<Vec<ProhibitedFile>> {
	let mut found = Vec::new();
	for (probe, folder) in [(mod_probe, "mods"), (pack_probe, "resourcepacks")]
	{
		let directory = probe.parent().expect("probe has a parent directory");
		let files = match fs::read_dir(directory) {
			Ok(files) => files,
			Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
			Err(error) => return Err(error.into()),
		};
		for entry in files {
			let entry = entry?;
			let name = entry.file_name();
			let Some(name) = name.to_str() else { continue };
			let file_type = entry.file_type()?;
			let is_pack = folder == "resourcepacks";
			let base = name.to_ascii_lowercase();
			let base = base.trim_end_matches(".disabled");
			if (is_pack
				&& (file_type.is_dir()
					|| file_type.is_symlink()
					|| base.ends_with(".zip")
					|| base.ends_with(".jar")))
				|| (!is_pack && base.ends_with(".jar"))
			{
				if let Some(content) = inspect_file(&entry.path(), is_pack)? {
					found.push(ProhibitedFile {
						relative: format!("{folder}/{name}"),
						content,
						is_directory: file_type.is_dir(),
					});
				}
			}
		}
	}
	Ok(found)
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::io::Write;
	use zip::write::SimpleFileOptions;

	fn archive(entries: &[(&str, &[u8])]) -> Vec<u8> {
		let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
		for (name, bytes) in entries {
			zip.start_file(*name, SimpleFileOptions::default()).unwrap();
			zip.write_all(bytes).unwrap();
		}
		zip.finish().unwrap().into_inner()
	}

	#[test]
	fn identifies_only_trollland_profile_names_and_known_downloads() {
		assert!(is_trollland_name("TrollLand"));
		assert!(is_trollland_name("TrollLand-Test"));
		assert!(is_trollland_name("TrollLand-1.32.115.mrpack"));
		assert!(!is_trollland_name("TrollLander"));
		assert!(!is_trollland_name("Other pack"));
		assert_eq!(
			prohibited_download(
				"https://cdn.modrinth.com/data/XeEZ3fK2/versions/id/renamed.jar"
			),
			Some(ProhibitedContent::Freecam)
		);
		assert_eq!(
			prohibited_download(
				"https://edge.forgecdn.net/files/6077/863/Xray_Ultimate_1.21_v5.2.1.zip"
			),
			Some(ProhibitedContent::Xray)
		);
		assert_eq!(prohibited_project("7QeJs94Z"), None);
	}

	#[test]
	fn finds_jar_in_jar_through_loader_metadata() {
		let nested = archive(&[(
			"META-INF/neoforge.mods.toml",
			b"[[mods]]\nmodId = 'freecam'",
		)]);
		let outer = archive(&[
			(
				"META-INF/jarjar/metadata.json",
				br#"{"jars":[{"path":"META-INF/deps/addon.jar"}]}"#,
			),
			("META-INF/deps/addon.jar", &nested),
		]);
		assert_eq!(
			inspect_bytes(&outer, "mods/renamed.jar"),
			Some(ProhibitedContent::Freecam)
		);
	}

	#[tokio::test]
	async fn removes_renamed_and_disabled_content_without_touching_other_files()
	{
		let root = tempfile::tempdir().unwrap();
		let mods = root.path().join("mods");
		let packs = root.path().join("resourcepacks");
		fs::create_dir(&mods).unwrap();
		fs::create_dir(&packs).unwrap();
		let freecam = archive(&[("fabric.mod.json", br#"{"id":"freecam"}"#)]);
		let harmless = archive(&[(
			"fabric.mod.json",
			br#"{"id":"anti_xray","depends":{"freecam":"*"}}"#,
		)]);
		let xray = archive(&[
			("pack.mcmeta", br#"{"pack":{"description":"Renamed"}}"#),
			("assets/minecraft/models/block/xray/stone.json", b"{}"),
			("assets/minecraft/models/block/xray/deepslate.json", b"{}"),
		]);
		fs::write(mods.join("addon.jar.disabled"), &freecam).unwrap();
		fs::write(mods.join("freecam.jar"), &harmless).unwrap();
		fs::write(packs.join("pack.zip"), xray).unwrap();
		let folder = packs.join("unpacked");
		fs::create_dir(&folder).unwrap();
		fs::write(folder.join("pack.mcmeta"), br#"{"pack":{"description":"Made By AlphaLeoli\nXray in spectator mode"}}"#).unwrap();
		let found = scan_import(root.path()).await.unwrap();
		assert_eq!(found.len(), 3);
		for file in found {
			let path = root.path().join(file.relative);
			if file.is_directory {
				remove_pack_directory(&path).await.unwrap();
			} else {
				unlink_file(&path).await.unwrap();
			}
		}
		assert!(scan_import(root.path()).await.unwrap().is_empty());
		assert_eq!(fs::read(mods.join("freecam.jar")).unwrap(), harmless);
	}
}
