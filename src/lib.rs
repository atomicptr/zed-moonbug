use core::net;
use std::{env, path::PathBuf};

use include_dir::{Dir, include_dir};
use serde::Serialize;
use zed_extension_api::{
    DebugAdapterBinary, DebugRequest, DebugScenario, StartDebuggingRequestArguments,
    StartDebuggingRequestArgumentsRequest, TcpArguments,
};

use crate::config::AdapterConfig;

mod config;

struct MoonbugDebugger;

impl zed_extension_api::Extension for MoonbugDebugger {
    fn new() -> Self
    where
        Self: Sized,
    {
        MoonbugDebugger
    }

    fn get_dap_binary(
        &mut self,
        adapter_name: String,
        config: zed_extension_api::DebugTaskDefinition,
        user_provided_debug_adapter_path: Option<String>,
        worktree: &zed_extension_api::Worktree,
    ) -> zed_extension_api::Result<zed_extension_api::DebugAdapterBinary, String> {
        if adapter_name != "moonbug" {
            return Err(format!("moonbug: unsupported adapter {adapter_name}"));
        }

        let cfg: AdapterConfig = serde_json::from_str(&config.config)
            .map_err(|e| format!("moonbug: debug config is not valid JSON: {e}"))?;

        let host: net::Ipv4Addr = cfg
            .host()
            .unwrap_or(config::DEFAULT_HOST.to_string())
            .parse()
            .map_err(|e| format!("moonbug: could not parse host: {e}"))?;
        let port: u16 = cfg.port().unwrap_or(config::DEFAULT_PORT);
        let project_root_dir = cfg
            .project_root_dir()
            .unwrap_or_else(|| worktree.root_path());

        let connection = Some(TcpArguments {
            host: host.to_bits(),
            port,
            timeout: Some(config::DEFAULT_TIMEOUT_MS),
        });

        let moonbug_config = MoonbugConfig {
            project_root_dir: project_root_dir.clone(),
        };

        let AdapterConfig::Launch {
            program, lua, args, ..
        } = &cfg
        else {
            // attach return early
            return Ok(DebugAdapterBinary {
                command: None,
                arguments: vec![],
                envs: vec![],
                cwd: Some(project_root_dir),
                connection,
                request_args: StartDebuggingRequestArguments {
                    configuration: serde_json::to_string(&moonbug_config)
                        .map_err(|e| format!("moonbug: could not serialize config: {e}"))?,
                    request: StartDebuggingRequestArgumentsRequest::Attach,
                },
            });
        };

        let program = program.clone();
        let lua = user_provided_debug_adapter_path
            .or_else(|| lua.clone())
            .or_else(|| worktree.which("lua"))
            .unwrap_or_else(|| "lua".to_string());

        let mut arguments = vec![program];
        arguments.extend(args.iter().cloned());

        let resources_dir = extension_install_dir()?
            .join("resources")
            .join(env!("CARGO_PKG_VERSION"));

        install_resources(&resources_dir)?;

        let resources_dir = resources_dir.to_string_lossy().to_string();

        Ok(DebugAdapterBinary {
            command: Some(lua),
            arguments,
            envs: vec![
                ("MOONBUG_PORT".into(), port.to_string()),
                (
                    "LUA_PATH".into(),
                    format!("{resources_dir}/?.lua;{resources_dir}/?/init.lua;;"),
                ),
            ],
            cwd: Some(project_root_dir),
            connection,
            request_args: StartDebuggingRequestArguments {
                configuration: serde_json::to_string(&moonbug_config)
                    .map_err(|e| format!("moonbug: could not serialize config: {e}"))?,
                request: StartDebuggingRequestArgumentsRequest::Launch,
            },
        })
    }

    fn dap_request_kind(
        &mut self,
        adapter_name: String,
        config: serde_json::Value,
    ) -> zed_extension_api::Result<StartDebuggingRequestArgumentsRequest, String> {
        if adapter_name != "moonbug" {
            return Err(format!("moonbug: unsupported adapter {adapter_name}"));
        }

        let cfg: AdapterConfig = serde_json::from_value(config)
            .map_err(|e| format!("moonbug: could not parse config: {e}"))?;

        match cfg {
            AdapterConfig::Launch { .. } => Ok(StartDebuggingRequestArgumentsRequest::Launch),
            AdapterConfig::Attach { .. } => Ok(StartDebuggingRequestArgumentsRequest::Attach),
        }
    }

    fn dap_config_to_scenario(
        &mut self,
        config: zed_extension_api::DebugConfig,
    ) -> zed_extension_api::Result<zed_extension_api::DebugScenario, String> {
        let cfg = match config.request {
            DebugRequest::Launch(req) => serde_json::json!({
                "request": "launch",
                "program": req.program,
                "args": req.args,
                "host": config::DEFAULT_HOST,
                "port": config::DEFAULT_PORT,
                "project_root_dir": req.cwd,
            }),
            DebugRequest::Attach(_) => serde_json::json!({
                "request": "attach",
                "host": config::DEFAULT_HOST,
                "port": config::DEFAULT_PORT,
            }),
        };

        Ok(DebugScenario {
            label: config.label,
            adapter: config.adapter,
            build: None,
            config: serde_json::to_string(&cfg)
                .map_err(|e| format!("moonbug: failed to serialize config: {e}"))?,
            tcp_connection: None,
        })
    }
}

zed_extension_api::register_extension!(MoonbugDebugger);

#[derive(Debug, Serialize)]
struct MoonbugConfig {
    project_root_dir: String,
}

fn extension_install_dir() -> Result<PathBuf, String> {
    env::current_dir().map_err(|e| format!("moonbug: could not determine extension directory: {e}"))
}

static RESOURCES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/resources");

fn install_resources(location: &PathBuf) -> Result<(), String> {
    std::fs::create_dir_all(location)
        .map_err(|e| format!("moonbug: could not create {}: {e}", location.display()))?;

    for file in RESOURCES.files() {
        let relative = file
            .path()
            .strip_prefix(RESOURCES.path())
            .map_err(|e| format!("moonbug: invalid resource path: {e}"))?;

        let dest = location.join(relative);

        std::fs::write(&dest, file.contents())
            .map_err(|e| format!("moonbug: could not write {}: {e}", dest.display()))?;
    }

    Ok(())
}
