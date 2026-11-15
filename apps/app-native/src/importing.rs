//! Импорт архивов сборок и установленных экземпляров других лаунчеров.

use crate::{
	app::App,
	backend::{Backend, Msg},
	theme::*,
	widgets::*,
};
use eframe::egui::{self, Id, Margin, Stroke};
use pack::import::ImportLauncherType;
use serde_json::{Value, json};
use std::{
	io::{Read, Write},
	path::{Path, PathBuf},
};
use theseus::{install, prelude::pack};

impl App {
	pub fn import_dialog(&mut self, ctx: &egui::Context) {
		if !self.import_open {
			return;
		}
		let response = egui::Modal::new(Id::new("import-pack"))
			.frame(egui::Frame::new().fill(PANEL()).corner_radius(14).stroke(Stroke::new(1.0, BORDER())).inner_margin(Margin::same(22)))
			.show(ctx, |ui| {
				ui.set_width(460.0);
				ui.spacing_mut().item_spacing.y = 12.0;
				label(ui, "Импорт сборки", bold(20.0), TEXT());
				label(ui, "Modrinth (.mrpack), CurseForge и Prism/MultiMC (.zip)", regular(13.0), TEXT_DIM());
				if button(ui, "Выбрать архив", Some(Icon::Folder), Kind::Primary, 36.0).clicked() {
					self.be.import_pack(false, self.prefs.cf_key.clone());
					self.import_open = false;
				}
				label(ui, "Или выберите папку конкретной сборки из Prism/MultiMC, CurseForge, ATLauncher или GDLauncher.", regular(13.0), TEXT_DIM());
				if button(ui, "Выбрать папку сборки", Some(Icon::Folder), Kind::Secondary, 36.0).clicked() {
					self.be.import_pack(true, self.prefs.cf_key.clone());
					self.import_open = false;
				}
				label(ui, "Для загрузки модов CurseForge из ZIP нужен API-ключ. Папки готовых сборок импортируются без ключа.", regular(12.5), TEXT_DIM());
				ui.collapsing("API-ключ CurseForge", |ui| {
					ui.add(egui::TextEdit::singleline(&mut self.cf_key_edit).hint_text("Вставьте ключ").password(true).desired_width(420.0));
					ui.horizontal(|ui| {
						if button(ui, "Сохранить ключ", None, Kind::Secondary, 30.0).clicked() && !self.cf_key_edit.trim().is_empty() {
							self.prefs.cf_key = self.cf_key_edit.trim().to_owned();
							crate::prefs::save(&self.prefs);
							self.categories.retain(|k, _| !k.starts_with("cf|"));
							self.browse_sig.clear();
						}
						ui.hyperlink_to("Получить ключ", "https://console.curseforge.com/");
					});
					if !self.prefs.cf_key.is_empty() { label(ui, "Ключ сохранён", regular(12.5), GREEN()); }
				});
				if button(ui, "Отмена", None, Kind::Secondary, 32.0).clicked() { self.import_open = false; }
			});
		if response.should_close() {
			self.import_open = false;
		}
	}
}

impl Backend {
	pub fn import_pack(&self, folder: bool, cf_key: String) {
		self.start_pack_import(folder, cf_key, None);
	}

	pub fn import_pack_path(&self, path: PathBuf, cf_key: String) {
		self.start_pack_import(path.is_dir(), cf_key, Some(path));
	}

	fn start_pack_import(
		&self,
		folder: bool,
		cf_key: String,
		path: Option<PathBuf>,
	) {
		let http = reqwest::Client::builder()
			.timeout(std::time::Duration::from_secs(60))
			.build()
			.expect("http");
		self.task(move |tx| async move {
			let path = match path {
				Some(path) => path,
				None => {
					let dialog = rfd::AsyncFileDialog::new();
					let selected = if folder {
						dialog
							.set_title("Выберите папку одной сборки")
							.pick_folder()
							.await
					} else {
						dialog
							.set_title("Импорт сборки")
							.add_filter("Сборки Minecraft", &["mrpack", "zip"])
							.pick_file()
							.await
					};
					let Some(selected) = selected else {
						return;
					};
					selected.path().to_path_buf()
				}
			};
			let _ = tx
				.send(Msg::Toast("Подготавливаю импорт сборки…".into(), false));
			let result = async {
				if folder {
					return import_folder(path).await;
				}
				let archive_path = path.clone();
				let kind = tokio::task::spawn_blocking(move || {
					archive_kind(&archive_path)
				})
				.await
				.map_err(message)??;
				if kind == ArchiveKind::Modrinth {
					return install::create_modpack_instance(
						pack::install_from::CreatePackLocation::FromFile {
							path,
						},
						None,
					)
					.await
					.map_err(message);
				}
				let staging = import_staging()?;
				let job = if kind == ArchiveKind::CurseForge {
					let source = path.clone();
					let manifest = tokio::task::spawn_blocking(move || {
						read_manifest(&source)
					})
					.await
					.map_err(message)??;
					let files =
						curseforge_files(&http, &cf_key, &manifest).await?;
					let output = staging.path().join("import.mrpack");
					let target = output.clone();
					tokio::task::spawn_blocking(move || {
						convert_curseforge(&path, &target, manifest, files)
					})
					.await
					.map_err(message)??;
					install::create_modpack_instance(
						pack::install_from::CreatePackLocation::FromFile {
							path: output,
						},
						None,
					)
					.await
					.map_err(message)?
				} else {
					let destination = staging.path().join("instances/imported");
					let dest = destination.clone();
					tokio::task::spawn_blocking(move || {
						extract_prism(&path, &dest)
					})
					.await
					.map_err(message)??;
					import_folder(destination).await?
				};
				// Задания выполняются движком в фоне; исходники нужны также для повтора после перезапуска.
				let _ = staging.keep();
				Ok(job)
			}
			.await;
			match result {
				Ok(_) => {
					let _ = tx.send(Msg::Toast(
						"Импорт запущен — прогресс в списке загрузок".into(),
						false,
					));
				}
				Err(error) => {
					let _ = tx.send(Msg::Toast(
						format!("Не удалось импортировать сборку: {error}"),
						true,
					));
				}
			}
			let _ = tx.send(Msg::InstallDone(String::new()));
		});
	}
}

fn message(error: impl std::fmt::Display) -> String {
	error.to_string()
}

#[derive(Debug, PartialEq, Eq)]
enum ArchiveKind {
	Modrinth,
	CurseForge,
	Prism,
}

fn archive_kind(path: &Path) -> Result<ArchiveKind, String> {
	let mut zip =
		zip::ZipArchive::new(std::fs::File::open(path).map_err(message)?)
			.map_err(message)?;
	if zip.by_name("modrinth.index.json").is_ok() {
		return Ok(ArchiveKind::Modrinth);
	}
	if zip.by_name("manifest.json").is_ok() {
		return Ok(ArchiveKind::CurseForge);
	}
	if prism_root(&zip).is_ok() {
		return Ok(ArchiveKind::Prism);
	}
	Err("Архив не содержит modrinth.index.json, manifest.json или сборку Prism/MultiMC".into())
}

async fn import_folder(
	path: PathBuf,
) -> Result<install::InstallJobSnapshot, String> {
	let types = [
		ImportLauncherType::PrismLauncher,
		ImportLauncherType::Curseforge,
		ImportLauncherType::ATLauncher,
		ImportLauncherType::GDLauncher,
	];
	for kind in types {
		if pack::import::is_valid_importable_instance(path.clone(), kind).await
		{
			let parent = path.parent().ok_or("Не найдена папка экземпляров")?;
			let expected = if kind == ImportLauncherType::Curseforge {
				"Instances"
			} else {
				"instances"
			};
			if parent.file_name().and_then(|s| s.to_str()) != Some(expected) {
				let staging = import_staging()?;
				let name = path
					.file_name()
					.ok_or("У папки нет имени")?
					.to_string_lossy()
					.to_string();
				let destination = staging.path().join(expected).join(&name);
				tokio::task::spawn_blocking(move || {
					copy_instance_folder(&path, &destination)
				})
				.await
				.map_err(message)??;
				let job = install::import_instance(
					kind,
					staging.path().to_path_buf(),
					name,
				)
				.await
				.map_err(message)?;
				let _ = staging.keep();
				return Ok(job);
			}
			let base = parent
				.parent()
				.ok_or("Не найдена папка лаунчера")?
				.to_path_buf();
			let name = path
				.file_name()
				.ok_or("У папки нет имени")?
				.to_string_lossy()
				.to_string();
			return install::import_instance(kind, base, name)
				.await
				.map_err(message);
		}
	}
	Err("Выберите папку одной сборки с mmc-pack.json, minecraftinstance.json, instance.json или config.json".into())
}

fn import_staging() -> Result<tempfile::TempDir, String> {
	let cache = dirs::cache_dir()
		.ok_or("Не найдена папка кэша")?
		.join("trolllauncher-rs/imports");
	std::fs::create_dir_all(&cache).map_err(message)?;
	tempfile::Builder::new()
		.prefix("pack-")
		.tempdir_in(cache)
		.map_err(message)
}

fn copy_instance_folder(source: &Path, target: &Path) -> Result<(), String> {
	std::fs::create_dir_all(target).map_err(message)?;
	for entry in std::fs::read_dir(source).map_err(message)? {
		let entry = entry.map_err(message)?;
		let kind = entry.file_type().map_err(message)?;
		if kind.is_symlink() {
			continue;
		}
		if kind.is_dir() {
			copy_instance_folder(
				&entry.path(),
				&target.join(entry.file_name()),
			)?;
		} else if kind.is_file() {
			std::fs::copy(entry.path(), target.join(entry.file_name()))
				.map_err(message)?;
		}
	}
	Ok(())
}

fn prism_root(
	archive: &zip::ZipArchive<std::fs::File>,
) -> Result<PathBuf, String> {
	let mut roots = archive.file_names().filter_map(|name| {
		let path = Path::new(name);
		if path.file_name()?.to_str()? != "mmc-pack.json" {
			return None;
		}
		let parent = path.parent()?;
		let config = parent
			.join("instance.cfg")
			.to_string_lossy()
			.replace('\\', "/");
		archive
			.file_names()
			.any(|n| n == config)
			.then(|| parent.to_path_buf())
	});
	let root = roots.next().ok_or("В архиве нет сборки Prism/MultiMC")?;
	if roots.next().is_some() {
		return Err(
			"В архиве несколько сборок — выберите архив одной сборки".into()
		);
	}
	Ok(root)
}

fn read_manifest(path: &Path) -> Result<Value, String> {
	let mut zip =
		zip::ZipArchive::new(std::fs::File::open(path).map_err(message)?)
			.map_err(message)?;
	let file = zip.by_name("manifest.json").map_err(message)?;
	serde_json::from_reader(file.take(4 * 1024 * 1024)).map_err(message)
}

fn dependencies(manifest: &Value) -> Result<Value, String> {
	if manifest["manifestType"] != "minecraftModpack"
		|| manifest["manifestVersion"] != 1
	{
		return Err("Неподдерживаемый манифест CurseForge".into());
	}
	let mc = manifest["minecraft"]["version"]
		.as_str()
		.filter(|s| !s.is_empty())
		.ok_or("В манифесте нет версии Minecraft")?;
	let mut deps = json!({"minecraft": mc});
	if let Some(loaders) = manifest["minecraft"]["modLoaders"].as_array() {
		let loader = loaders
			.iter()
			.find(|l| l["primary"] == true)
			.or_else(|| loaders.first());
		if let Some(loader) = loader {
			let id =
				loader["id"].as_str().ok_or("Не указан загрузчик модов")?;
			let (key, version) = [
				("neoforge-", "neoforge"),
				("forge-", "forge"),
				("fabric-", "fabric-loader"),
				("quilt-", "quilt-loader"),
			]
			.into_iter()
			.find_map(|(prefix, key)| id.strip_prefix(prefix).map(|v| (key, v)))
			.ok_or_else(|| format!("Неизвестный загрузчик: {id}"))?;
			if version.is_empty() {
				return Err("Не указана версия загрузчика".into());
			}
			deps[key] = json!(version);
		}
	}
	Ok(deps)
}

async fn curseforge_files(
	http: &reqwest::Client,
	key: &str,
	manifest: &Value,
) -> Result<Vec<Value>, String> {
	dependencies(manifest)?;
	let entries = manifest["files"]
		.as_array()
		.ok_or("В манифесте нет списка files")?;
	if !entries.is_empty() && key.trim().is_empty() {
		return Err("Сохраните API-ключ CurseForge в окне импорта, затем выберите архив ещё раз".into());
	}
	let mut files = Vec::new();
	for entry in entries {
		let project =
			entry["projectID"].as_u64().ok_or("Отсутствует projectID")?;
		let id = entry["fileID"].as_u64().ok_or("Отсутствует fileID")?;
		let response: Value = http
			.get(format!(
				"https://api.curseforge.com/v1/mods/{project}/files/{id}"
			))
			.header("x-api-key", key.trim())
			.send()
			.await
			.map_err(message)?
			.error_for_status()
			.map_err(message)?
			.json()
			.await
			.map_err(message)?;
		let data = &response["data"];
		let name = data["fileName"]
			.as_str()
			.filter(|n| {
				!n.is_empty()
					&& !n.contains(['/', '\\'])
					&& *n != "." && *n != ".."
			})
			.ok_or("Некорректное имя файла CurseForge")?;
		let url = data["downloadUrl"].as_str().filter(|s| s.starts_with("https://")).ok_or_else(|| format!("Автор запретил автоматическую загрузку {name} ({project}/{id}). Импортируйте установленную папку сборки из CurseForge."))?;
		let hash = data["hashes"]
			.as_array()
			.and_then(|h| h.iter().find(|h| h["algo"] == 1))
			.and_then(|h| h["value"].as_str())
			.ok_or("CurseForge не вернул SHA-1 файла")?;
		if hash.len() != 40 || !hash.bytes().all(|c| c.is_ascii_hexdigit()) {
			return Err("CurseForge вернул некорректный SHA-1".into());
		}
		let size = data["fileLength"]
			.as_u64()
			.filter(|s| *s <= u32::MAX as u64)
			.ok_or("Некорректный размер файла")?;
		let project_data: Value = http
			.get(format!("https://api.curseforge.com/v1/mods/{project}"))
			.header("x-api-key", key.trim())
			.send()
			.await
			.map_err(message)?
			.error_for_status()
			.map_err(message)?
			.json()
			.await
			.map_err(message)?;
		let folder = match project_data["data"]["classId"].as_u64() {
			Some(6) => "mods",
			Some(12) => "resourcepacks",
			Some(6552) => "shaderpacks",
			other => {
				return Err(format!(
					"Неподдерживаемый тип файла {name}: {other:?}"
				));
			}
		};
		files.push(json!({"path": format!("{folder}/{name}"), "hashes": {"sha1": hash}, "downloads": [url], "fileSize": size}));
	}
	Ok(files)
}

fn safe_archive_path(
	file: &zip::read::ZipFile<'_, std::fs::File>,
) -> Result<PathBuf, String> {
	if file.name().contains('\\')
		|| file.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000)
	{
		return Err(
			"Архив содержит небезопасный путь или символическую ссылку".into(),
		);
	}
	file.enclosed_name()
		.ok_or_else(|| "Архив содержит путь за пределами папки сборки".into())
}

fn convert_curseforge(
	source: &Path,
	target: &Path,
	manifest: Value,
	files: Vec<Value>,
) -> Result<(), String> {
	let deps = dependencies(&manifest)?;
	let overrides = manifest["overrides"].as_str().unwrap_or("overrides");
	let prefix = Path::new(overrides);
	if prefix.as_os_str().is_empty()
		|| prefix.is_absolute()
		|| overrides.contains('\\')
		|| prefix
			.components()
			.any(|c| !matches!(c, std::path::Component::Normal(_)))
	{
		return Err("Некорректная папка overrides".into());
	}
	let index = json!({"formatVersion": 1, "game": "minecraft", "versionId": manifest["version"].as_str().unwrap_or("1"), "name": manifest["name"].as_str().unwrap_or("CurseForge import"), "files": files, "dependencies": deps});
	let mut input =
		zip::ZipArchive::new(std::fs::File::open(source).map_err(message)?)
			.map_err(message)?;
	let mut output =
		zip::ZipWriter::new(std::fs::File::create(target).map_err(message)?);
	let options = zip::write::SimpleFileOptions::default()
		.compression_method(zip::CompressionMethod::Deflated);
	output
		.start_file("modrinth.index.json", options)
		.map_err(message)?;
	output
		.write_all(serde_json::to_string(&index).map_err(message)?.as_bytes())
		.map_err(message)?;
	for i in 0..input.len() {
		let mut file = input.by_index(i).map_err(message)?;
		let path = safe_archive_path(&file)?;
		if file.is_dir() {
			continue;
		}
		if let Ok(relative) = path.strip_prefix(prefix) {
			if relative.as_os_str().is_empty() {
				continue;
			}
			output
				.start_file(
					format!(
						"overrides/{}",
						relative.to_string_lossy().replace('\\', "/")
					),
					options,
				)
				.map_err(message)?;
			std::io::copy(&mut file, &mut output).map_err(message)?;
		}
	}
	output.finish().map_err(message)?;
	Ok(())
}

fn extract_prism(source: &Path, destination: &Path) -> Result<(), String> {
	let mut archive =
		zip::ZipArchive::new(std::fs::File::open(source).map_err(message)?)
			.map_err(message)?;
	let root = prism_root(&archive)?;
	for i in 0..archive.len() {
		let mut file = archive.by_index(i).map_err(message)?;
		let safe = safe_archive_path(&file)?;
		let Ok(relative) = safe.strip_prefix(&root) else {
			continue;
		};
		if relative.as_os_str().is_empty() {
			continue;
		}
		let path = destination.join(relative);
		if file.is_dir() {
			std::fs::create_dir_all(&path).map_err(message)?;
		} else {
			std::fs::create_dir_all(path.parent().ok_or("Некорректный путь")?)
				.map_err(message)?;
			let mut output = std::fs::File::create(path).map_err(message)?;
			std::io::copy(&mut file, &mut output).map_err(message)?;
		}
	}
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	fn archive(path: &Path, entries: &[(&str, &str)]) {
		let mut zip = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
		for (name, body) in entries {
			zip.start_file(*name, zip::write::SimpleFileOptions::default())
				.unwrap();
			zip.write_all(body.as_bytes()).unwrap();
		}
		zip.finish().unwrap();
	}

	fn manifest() -> Value {
		json!({"manifestType": "minecraftModpack", "manifestVersion": 1, "name": "Test pack", "version": "2", "minecraft": {"version": "1.21.1", "modLoaders": [{"id": "neoforge-21.1.1", "primary": true}]}, "files": [], "overrides": "overrides"})
	}

	#[test]
	fn curseforge_conversion_preserves_dependencies_and_game_files() {
		let temp = tempfile::tempdir().unwrap();
		let input = temp.path().join("pack.zip");
		let output = temp.path().join("pack.mrpack");
		archive(
			&input,
			&[
				("manifest.json", &manifest().to_string()),
				("overrides/config/test.toml", "enabled=true"),
				("overrides/saves/world/level.dat", "world"),
				("overrides/mods/local.jar", "local"),
				("README.txt", "outside overrides"),
			],
		);
		assert_eq!(archive_kind(&input).unwrap(), ArchiveKind::CurseForge);
		let files = vec![
			json!({"path": "mods/test.jar", "hashes": {"sha1": "0123456789012345678901234567890123456789"}, "downloads": ["https://example.com/test.jar"], "fileSize": 10}),
		];
		convert_curseforge(
			&input,
			&output,
			read_manifest(&input).unwrap(),
			files,
		)
		.unwrap();
		assert_eq!(archive_kind(&output).unwrap(), ArchiveKind::Modrinth);
		let mut zip =
			zip::ZipArchive::new(std::fs::File::open(output).unwrap()).unwrap();
		let parsed: pack::install_from::PackFormat = serde_json::from_reader(
			zip.by_name("modrinth.index.json").unwrap(),
		)
		.unwrap();
		assert_eq!(parsed.name, "Test pack");
		assert_eq!(
			parsed.dependencies[&pack::install_from::PackDependency::NeoForge],
			"21.1.1"
		);
		assert_eq!(parsed.files.len(), 1);
		assert!(zip.by_name("overrides/config/test.toml").is_ok());
		assert!(zip.by_name("overrides/saves/world/level.dat").is_ok());
		assert!(zip.by_name("overrides/mods/local.jar").is_ok());
		assert!(zip.by_name("README.txt").is_err());
	}

	#[test]
	fn loader_versions_are_preserved_and_unknown_loader_is_rejected() {
		for (id, key, version) in [
			("forge-47.3.0", "forge", "47.3.0"),
			("fabric-0.16.9", "fabric-loader", "0.16.9"),
			("quilt-0.27.1", "quilt-loader", "0.27.1"),
		] {
			let mut m = manifest();
			m["minecraft"]["modLoaders"] = json!([{"id": id}]);
			assert_eq!(dependencies(&m).unwrap()[key], version);
		}
		let mut m = manifest();
		m["minecraft"]["modLoaders"] = json!([]);
		assert_eq!(dependencies(&m).unwrap(), json!({"minecraft": "1.21.1"}));
		m["minecraft"]["modLoaders"] = json!([{"id": "unknown-1"}]);
		assert!(dependencies(&m).is_err());
		m["manifestVersion"] = json!(2);
		assert!(dependencies(&m).is_err());
	}

	#[test]
	fn prism_archive_preserves_instance_and_world() {
		let temp = tempfile::tempdir().unwrap();
		let input = temp.path().join("prism.zip");
		let dest = temp.path().join("unpacked");
		archive(
			&input,
			&[
				("mmc-pack.json", "{\"components\":[]}"),
				("instance.cfg", "[General]\nname=Example\n"),
				(".minecraft/saves/world/level.dat", "world"),
			],
		);
		assert_eq!(archive_kind(&input).unwrap(), ArchiveKind::Prism);
		extract_prism(&input, &dest).unwrap();
		assert_eq!(
			std::fs::read_to_string(
				dest.join(".minecraft/saves/world/level.dat")
			)
			.unwrap(),
			"world"
		);
	}

	#[test]
	fn unsafe_archive_paths_and_overrides_are_rejected() {
		let temp = tempfile::tempdir().unwrap();
		for name in ["../escape", "/absolute", "overrides\\..\\escape"] {
			let input = temp.path().join("bad.zip");
			archive(
				&input,
				&[
					("mmc-pack.json", "{}"),
					("instance.cfg", "name=Test"),
					(name, "bad"),
				],
			);
			assert!(extract_prism(&input, &temp.path().join("dest")).is_err());
			assert!(!temp.path().join("escape").exists());
		}
		let input = temp.path().join("empty.zip");
		archive(&input, &[]);
		assert!(archive_kind(&input).is_err());
		let mut m = manifest();
		m["overrides"] = json!("../outside");
		assert!(
			convert_curseforge(
				&input,
				&temp.path().join("bad.mrpack"),
				m,
				vec![]
			)
			.is_err()
		);
	}

	#[test]
	fn wrapped_prism_archive_and_detached_folder_keep_contents() {
		let temp = tempfile::tempdir().unwrap();
		let input = temp.path().join("wrapped.zip");
		archive(
			&input,
			&[
				("Example/mmc-pack.json", "{}"),
				("Example/instance.cfg", "name=Example"),
				("Example/.minecraft/mods/test.jar", "mod"),
				("outside.txt", "ignore"),
			],
		);
		assert_eq!(archive_kind(&input).unwrap(), ArchiveKind::Prism);
		let unpacked = temp.path().join("unpacked");
		extract_prism(&input, &unpacked).unwrap();
		assert!(!unpacked.join("outside.txt").exists());
		assert_eq!(
			std::fs::read_to_string(unpacked.join(".minecraft/mods/test.jar"))
				.unwrap(),
			"mod"
		);
		let copied = temp.path().join("copied");
		copy_instance_folder(&unpacked, &copied).unwrap();
		assert_eq!(
			std::fs::read_to_string(copied.join(".minecraft/mods/test.jar"))
				.unwrap(),
			"mod"
		);
		assert!(unpacked.join("instance.cfg").exists());
	}

	#[test]
	fn archive_symlinks_are_rejected() {
		let temp = tempfile::tempdir().unwrap();
		let input = temp.path().join("symlink.zip");
		let mut zip =
			zip::ZipWriter::new(std::fs::File::create(&input).unwrap());
		zip.add_symlink(
			"overrides/link",
			"../../outside",
			zip::write::SimpleFileOptions::default(),
		)
		.unwrap();
		zip.finish().unwrap();
		assert!(
			convert_curseforge(
				&input,
				&temp.path().join("bad.mrpack"),
				manifest(),
				vec![]
			)
			.is_err()
		);
	}

	#[tokio::test]
	async fn empty_curseforge_pack_needs_no_key_but_downloads_do() {
		let mut m = manifest();
		let http = reqwest::Client::new();
		assert!(curseforge_files(&http, "", &m).await.unwrap().is_empty());
		m["files"] = json!([{"projectID": 1, "fileID": 2}]);
		assert!(
			curseforge_files(&http, "", &m)
				.await
				.unwrap_err()
				.contains("API-ключ")
		);
	}
}
