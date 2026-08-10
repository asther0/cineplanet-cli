use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

use crate::domain::Preferences;

pub fn preferences_path() -> Result<PathBuf> {
    if let Some(directory) = std::env::var_os("CINEPLANET_CONFIG_DIR") {
        return Ok(PathBuf::from(directory).join("preferences.json"));
    }
    let project = directories::ProjectDirs::from("dev", "asther0", "CineplanetCLI")
        .context("macOS no proporcionó un directorio de configuración válido")?;
    Ok(project.config_dir().join("preferences.json"))
}

pub fn load() -> Result<Preferences> {
    load_or_default_from(&preferences_path()?)
}

pub fn save(preferences: &Preferences) -> Result<()> {
    save_to(&preferences_path()?, preferences)
}

pub fn load_from(path: &Path) -> Result<Preferences> {
    let bytes = fs::read(path)
        .with_context(|| format!("no se pudieron leer preferencias desde {}", path.display()))?;
    serde_json::from_slice(&bytes)
        .with_context(|| format!("preferencias inválidas en {}", path.display()))
}

pub fn load_or_default_from(path: &Path) -> Result<Preferences> {
    if path.exists() {
        load_from(path)
    } else {
        Ok(Preferences::default())
    }
}

pub fn save_to(path: &Path, preferences: &Preferences) -> Result<()> {
    let parent = path
        .parent()
        .context("la ruta de preferencias no tiene directorio padre")?;
    fs::create_dir_all(parent).with_context(|| {
        format!(
            "no se pudo crear el directorio de preferencias {}",
            parent.display()
        )
    })?;

    let bytes = serde_json::to_vec_pretty(preferences)?;
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, bytes).with_context(|| {
        format!(
            "no se pudieron escribir preferencias temporales en {}",
            temporary.display()
        )
    })?;
    fs::rename(&temporary, path)
        .with_context(|| format!("no se pudieron guardar preferencias en {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeSet, fs, process, time::SystemTime};

    use crate::domain::Preferences;

    use super::{load_from, load_or_default_from, save_to};

    #[test]
    fn saved_preferences_round_trip() {
        let unique = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "cineplanet-cli-settings-{}-{unique}",
            process::id()
        ));
        let path = root.join("preferences.json");
        let preferences = Preferences {
            onboarding_complete: true,
            party_size: 4,
            favorite_venue_ids: BTreeSet::from(["la-molina".into(), "risso".into()]),
            city: None,
            accepted_languages: BTreeSet::from(["Subtitulada".into()]),
            accepted_formats: BTreeSet::from(["2D".into()]),
            accepted_room_types: BTreeSet::from(["Regular".into(), "Prime".into()]),
        };

        save_to(&path, &preferences).unwrap();
        let loaded = load_from(&path).unwrap();

        assert_eq!(loaded, preferences);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn missing_preferences_use_the_product_defaults() {
        let path =
            std::env::temp_dir().join(format!("cineplanet-cli-missing-settings-{}", process::id()));

        let loaded = load_or_default_from(&path).unwrap();

        assert_eq!(loaded, Preferences::default());
    }

    #[test]
    fn preferences_from_older_versions_receive_missing_defaults() {
        let unique = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "cineplanet-cli-legacy-settings-{}-{unique}",
            process::id()
        ));
        let path = root.join("preferences.json");
        fs::create_dir_all(&root).unwrap();
        fs::write(
            &path,
            r#"{
                "party_size": 2,
                "favorite_venue_ids": [],
                "accepted_languages": [],
                "accepted_formats": [],
                "accepted_room_types": []
            }"#,
        )
        .unwrap();

        let loaded = load_from(&path).unwrap();

        assert!(!loaded.onboarding_complete);
        fs::remove_dir_all(root).unwrap();
    }
}
