use crate::{AuditEvent, AuditLogger};
use serde::Serialize;
use serde_json::{json, Value};
use std::{collections::BTreeMap, ffi::OsString, path::Path};

pub const ENVIRONMENT_KEYS: &[&str] = &[
    "APPIMAGE",
    "APPDIR",
    "PATH",
    "LD_LIBRARY_PATH",
    "LD_PRELOAD",
    "GTK_PATH",
    "GTK_EXE_PREFIX",
    "GTK_DATA_PREFIX",
    "GIO_EXTRA_MODULES",
    "GIO_MODULE_DIR",
    "XDG_DATA_DIRS",
    "XDG_CONFIG_DIRS",
    "XDG_CURRENT_DESKTOP",
    "XDG_SESSION_DESKTOP",
    "XDG_SESSION_TYPE",
    "DESKTOP_SESSION",
    "DISPLAY",
    "WAYLAND_DISPLAY",
    "XDG_RUNTIME_DIR",
    "DBUS_SESSION_BUS_ADDRESS",
    "LANG",
    "LC_ALL",
    "LC_CTYPE",
    "RUST_LOG",
    "PATHEXT",
    "SystemRoot",
    "WINDIR",
    "DYLD_LIBRARY_PATH",
    "DYLD_INSERT_LIBRARIES",
];
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct EnvironmentSnapshot {
    pub(crate) variables: BTreeMap<String, Value>,
}
impl AuditLogger {
    /// Caller collects OS state; the core only transforms explicit values.
    pub fn environment(&self, values: &BTreeMap<String, OsString>) -> EnvironmentSnapshot {
        let mut inner = self.owner.shared.lock();
        let appdir = values.get("APPDIR").and_then(|v| v.to_str());
        let mut variables = BTreeMap::new();
        for &name in ENVIRONMENT_KEYS {
            let value = match values.get(name) {
                None => json!({"state":"unset"}),
                Some(v) => match v.to_str() {
                    None => json!({"state":"invalidEncoding"}),
                    Some("") => json!({"state":"empty"}),
                    Some(s) if s.len() > 8192 => json!({"state":"truncated","bytes":s.len()}),
                    Some(s) => {
                        let detail = match name {
                            "RUST_LOG" => json!({}),
                            "DBUS_SESSION_BUS_ADDRESS" => {
                                json!({"transport":if s.starts_with("unix:"){"unix"}else if s.starts_with("tcp:"){"tcp"}else{"unknown"}})
                            }
                            "XDG_CURRENT_DESKTOP"
                            | "XDG_SESSION_DESKTOP"
                            | "XDG_SESSION_TYPE"
                            | "DESKTOP_SESSION" => json!({"value":desktop(s)}),
                            "LANG" | "LC_ALL" | "LC_CTYPE" => json!({"value":locale(s)}),
                            "PATHEXT" => {
                                json!({"extensions":s.split(';').take(32).map(|x|match x.to_ascii_uppercase().as_str(){".EXE"=>".EXE",".COM"=>".COM",".BAT"=>".BAT",".CMD"=>".CMD",_=>"unknown"}).collect::<Vec<_>>()})
                            }
                            "DISPLAY" | "WAYLAND_DISPLAY" => {
                                json!({"reference":inner.privacy.alias(s)})
                            }
                            "PATH"
                            | "LD_LIBRARY_PATH"
                            | "GTK_PATH"
                            | "GIO_EXTRA_MODULES"
                            | "XDG_DATA_DIRS"
                            | "XDG_CONFIG_DIRS"
                            | "DYLD_LIBRARY_PATH"
                            | "DYLD_INSERT_LIBRARIES"
                            | "LD_PRELOAD" => {
                                let separator = if cfg!(windows) && name == "PATH" {
                                    ';'
                                } else {
                                    ':'
                                };
                                let entries: Vec<_> = if name == "LD_PRELOAD" {
                                    s.split(|c: char| c == ':' || c.is_ascii_whitespace())
                                        .collect()
                                } else {
                                    s.split(separator).collect()
                                };
                                json!({"entries":entries.iter().take(256).map(|p|inner.privacy.path(p,appdir)).collect::<Vec<_>>(),"omittedEntries":entries.len().saturating_sub(256)})
                            }
                            _ => inner.privacy.path(s, appdir),
                        };
                        json!({"state":"present","detail":detail})
                    }
                },
            };
            variables.insert(name.into(), value);
        }
        // Each variable is bounded; also bound the combined snapshot without cutting JSON.
        while serde_json::to_vec(&variables).map_or(usize::MAX, |v| v.len()) > 6 * 1024 {
            if let Some((_, largest)) = variables
                .iter_mut()
                .max_by_key(|(_, v)| serde_json::to_vec(v).map_or(0, |b| b.len()))
            {
                if let Some(entries) = largest
                    .get_mut("detail")
                    .and_then(|v| v.get_mut("entries"))
                    .and_then(Value::as_array_mut)
                    .filter(|v| !v.is_empty())
                {
                    entries.pop();
                    let count = largest["detail"]["omittedEntries"].as_u64().unwrap_or(0);
                    largest["detail"]["omittedEntries"] = json!(count + 1);
                } else {
                    *largest = json!({"state":"truncated"});
                }
            } else {
                break;
            }
        }
        EnvironmentSnapshot { variables }
    }
    pub fn startup_metadata(
        &self,
        version: &str,
        revision: Option<&str>,
        executable: &Path,
        cwd: &Path,
        environment: EnvironmentSnapshot,
    ) {
        let version = if version.len() <= 64
            && version
                .chars()
                .all(|c| c.is_ascii_digit() || ".-rc".contains(c))
        {
            version
        } else {
            "unknown"
        };
        let revision = revision
            .filter(|r| r.len() <= 64 && r.chars().all(|c| c.is_ascii_hexdigit()))
            .unwrap_or("unknown");
        let executable = self.path_reference(executable);
        let cwd = self.path_reference(cwd);
        let packaged = environment
            .variables
            .get("APPIMAGE")
            .is_some_and(|v| v["state"] == "present");
        self.owner.shared.lock().metadata = json!({"schemaVersion":1,"runId":self.run_id(),"appVersion":version,"buildRevision":revision,"os":std::env::consts::OS,"architecture":std::env::consts::ARCH,"executable":executable,"workingDirectory":cwd,"packageKind":if packaged{"appImage"}else{"unknown"},"packageDetection":"environment","environment":environment,"environmentObservation":"process_after_launch_wrappers","createdAt":crate::now_string()});
        self.owner.shared.wake.notify_one();
    }
    /// Only the calling process is observable; the plugin owns helper selection and dispatch.
    pub fn opener_environment(&self, cwd: &Path, parent: EnvironmentSnapshot) {
        let startup = self
            .owner
            .shared
            .lock()
            .metadata
            .get("environment")
            .cloned()
            .unwrap_or(Value::Null);
        let changed = serde_json::to_value(&parent).ok().as_ref() != Some(&startup);
        let mut event = AuditEvent::succeeded("desktop.launch", "");
        event.stage = "configuration".into();
        event.audience = "support".into();
        event.details = json!({"workingDirectory":self.path_reference(cwd),"parentEnvironment":parent,"startupEnvironmentChanged":changed,
            "environmentObservation":"parent_at_opener_call","dispatch":"tauri_opener","childEnvironment":"not_observed","selectedHelper":"not_observed","desktopCompletion":"not_observed"});
        self.trusted(event);
    }
}
fn desktop(s: &str) -> &str {
    match s {
        "GNOME" | "KDE" | "XFCE" | "Xfce" | "LXDE" | "LXQt" | "Unity" | "Cinnamon" | "MATE"
        | "sway" | "Hyprland" | "ubuntu:GNOME" | "gnome" | "plasma" | "wayland" | "x11" | "tty" => {
            s
        }
        _ => "unknown",
    }
}
fn locale(s: &str) -> &str {
    match s {
        "C" | "POSIX" | "C.UTF-8" | "en_US.UTF-8" | "en_GB.UTF-8" | "ja_JP.UTF-8"
        | "ko_KR.UTF-8" | "zh_CN.UTF-8" | "zh_TW.UTF-8" => s,
        _ => "other",
    }
}

impl AuditLogger {
    /// Platform adapter supplies only OS metadata; allowlists are enforced here too.
    pub fn platform_metadata(
        &self,
        os_version: Option<&str>,
        distribution: Option<&str>,
        distribution_version: Option<&str>,
    ) {
        fn version(value: Option<&str>) -> Option<&str> {
            value.filter(|v| {
                !v.is_empty() && v.len() <= 64 && v.chars().all(|c| c.is_ascii_digit() || c == '.')
            })
        }
        let distro = distribution.filter(|v| {
            matches!(
                *v,
                "ubuntu"
                    | "debian"
                    | "fedora"
                    | "arch"
                    | "linuxmint"
                    | "opensuse-tumbleweed"
                    | "opensuse-leap"
                    | "pop"
                    | "nixos"
                    | "manjaro"
                    | "rhel"
                    | "centos"
                    | "alpine"
            )
        });
        let mut inner = self.owner.shared.lock();
        inner.metadata["osVersion"] = json!(version(os_version));
        inner.metadata["distribution"] = json!(distro.unwrap_or("unknown"));
        inner.metadata["distributionVersion"] = json!(version(distribution_version));
        inner.metadata["capabilities"] = json!({"ownedHelperObservation":false,"outputPolicy":"not_collected","frontendStacks":"omitted","legacyLogs":"excluded"});
        self.owner.shared.wake.notify_one();
    }
}
