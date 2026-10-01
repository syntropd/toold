//! Handler for virtual HID actuator commands.

use crate::cli::ActuatorCommands;
use crate::client::TooldClient;
use anyhow::Result;
use serde_json::json;

/// Executes the `actuator` subcommand suite.
pub async fn exec_actuator(
    client: &TooldClient,
    cmd: &ActuatorCommands,
    as_json: bool,
) -> Result<()> {
    match cmd {
        ActuatorCommands::Key { code, up } => {
            let down = !up;
            let params = json!({
                "key_code": code,
                "down": down,
            });
            let reply = client
                .call("io.syntrop.Actuator1.SendKey", Some(params))
                .await?;
            if as_json {
                println!("{}", serde_json::to_string_pretty(&reply)?);
            } else {
                println!(
                    "Emitted key {} ({})",
                    code,
                    if down { "down" } else { "up" }
                );
            }
        }
        ActuatorCommands::Type { text } => {
            let params = json!({
                "text": text,
            });
            let reply = client
                .call("io.syntrop.Actuator1.TypeText", Some(params))
                .await?;
            if as_json {
                println!("{}", serde_json::to_string_pretty(&reply)?);
            } else {
                println!("Typed text ({} chars)", text.chars().count());
            }
        }
        ActuatorCommands::Move { dx, dy } => {
            let params = json!({
                "dx": dx,
                "dy": dy,
            });
            let reply = client
                .call("io.syntrop.Actuator1.MoveMouse", Some(params))
                .await?;
            if as_json {
                println!("{}", serde_json::to_string_pretty(&reply)?);
            } else {
                println!("Moved mouse by ({}, {})", dx, dy);
            }
        }
        ActuatorCommands::Click { button } => {
            let params = json!({
                "button": button,
            });
            let reply = client
                .call("io.syntrop.Actuator1.ClickMouse", Some(params))
                .await?;
            if as_json {
                println!("{}", serde_json::to_string_pretty(&reply)?);
            } else {
                println!("Clicked mouse button {}", button);
            }
        }
    }

    Ok(())
}
