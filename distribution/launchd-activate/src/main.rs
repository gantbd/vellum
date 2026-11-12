use std::{path::PathBuf, process::ExitCode};

use vellum_launchd_activate::{
    MacOsLaunchdPortV1, activate_v1, first_install_v1, read_configuration_file_v1,
    read_first_install_configuration_file_v1, reconcile_activation_journal_v1,
    reconcile_first_install_journal_v1,
};

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args_os().skip(1);
    let Some(first) = arguments.next() else {
        return Err("usage: vellum-launchd-activate EFFECT_SWITCH --config=PRIVATE.json".into());
    };
    if first == "--help" && arguments.next().is_none() {
        println!(
            "vellum-launchd-activate (--execute-authorized-first-install | --execute-authorized-activation | --reconcile-first-install-journal | --reconcile-journal) --config=ABSOLUTE_PRIVATE.json"
        );
        return Ok(());
    }
    if first != "--execute-authorized-first-install"
        && first != "--execute-authorized-activation"
        && first != "--reconcile-first-install-journal"
        && first != "--reconcile-journal"
    {
        return Err("explicit activation or reconciliation switch is required".into());
    }
    let config = arguments
        .next()
        .ok_or("usage: vellum-launchd-activate EFFECT_SWITCH --config=PRIVATE.json")?;
    if arguments.next().is_some() {
        return Err("unexpected activation argument".into());
    }
    let config = config
        .to_str()
        .and_then(|value| value.strip_prefix("--config="))
        .ok_or("activation configuration argument is incompatible")?;
    let config = PathBuf::from(config);
    if first == "--execute-authorized-first-install" || first == "--reconcile-first-install-journal"
    {
        let configuration = read_first_install_configuration_file_v1(&config)?;
        if first == "--reconcile-first-install-journal" {
            let report = reconcile_first_install_journal_v1(&configuration)?;
            println!("{}", serde_json::to_string(&report)?);
            return Ok(());
        }
        let mut port = MacOsLaunchdPortV1::try_new(configuration.owner_uid)
            .ok_or("macOS launchd first installation is unavailable for this owner")?;
        let receipt = first_install_v1(&configuration, &mut port)?;
        println!("{}", serde_json::to_string(&receipt)?);
        return Ok(());
    }
    let configuration = read_configuration_file_v1(&config)?;
    if first == "--reconcile-journal" {
        let report = reconcile_activation_journal_v1(&configuration)?;
        println!("{}", serde_json::to_string(&report)?);
        return Ok(());
    }
    let mut port = MacOsLaunchdPortV1::try_new(configuration.owner_uid)
        .ok_or("macOS launchd activation is unavailable for this owner")?;
    let receipt = activate_v1(&configuration, &mut port)?;
    println!("{}", serde_json::to_string(&receipt)?);
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("vellum-launchd-activate: {error}");
            ExitCode::from(2)
        }
    }
}
