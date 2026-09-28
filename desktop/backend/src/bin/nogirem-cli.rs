use nogirem_backend::{
    Result,
    affinity::Manager,
    graphics, memory, muo,
    network::{self, Operations},
    nic, process, storage,
};
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};
static STOP: AtomicBool = AtomicBool::new(false);
unsafe extern "system" fn console_signal(kind: u32) -> i32 {
    if kind <= 6 {
        STOP.store(true, Ordering::SeqCst);
        1
    } else {
        0
    }
}
fn print(value: Value) {
    println!("{}", serde_json::to_string_pretty(&value).unwrap());
}
fn run() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let has = |s: &str| args.iter().any(|a| a == s);
    let apply = has("--apply");
    if has("--last-core") && has("--passive") {
        return Err("Choose only one mode: --last-core or --passive.".into());
    }
    if has("--reset-all") && (has("--last-core") || has("--passive")) {
        return Err("--reset-all cannot be combined with another mode.".into());
    }
    if !apply
        && [
            "--fast-ping-restart",
            "--nic-rss-apply",
            "--nic-rss-restore",
        ]
        .iter()
        .any(|a| has(a))
    {
        return Err("This operation requires --apply.".into());
    }
    let root = std::env::var_os("NOGIREM_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."));
    let config = storage::read_json(&root.join("config.json"))?.ok_or("config.json missing")?;
    let game = config["gameExecutable"].as_str().unwrap_or("");
    if has("--muo-status") {
        let directory = if let Some(i) = args.iter().position(|a| a == "--dir") {
            PathBuf::from(args.get(i + 1).ok_or("--dir requires a path")?)
        } else {
            let documents = nogirem_backend::powershell::run(
                "[Environment]::GetFolderPath('MyDocuments')",
                15000,
                16384,
            )?;
            PathBuf::from(documents.trim()).join("마비노기/설정")
        };
        print(muo::latest(&directory)?);
        return Ok(());
    }
    if has("--graphics-status") || has("--graphics-apply") {
        print(graphics::run(&root, game, has("--graphics-apply"))?);
        return Ok(());
    }
    if has("--nvidia-status") || has("--nvidia-apply") {
        print(graphics::nvidia(game, has("--nvidia-apply"))?);
        return Ok(());
    }
    if has("--fast-ping-status") || has("--fast-ping-restart") {
        let mut ops = network::Windows;
        let mut value = network::ensure_fast(&mut ops, apply, true)?;
        if has("--fast-ping-restart") && value["restarted"] != true {
            value["restart"] = ops.restart(&value["current"])?;
            value["restarted"] = json!(true);
        }
        print(value);
        return Ok(());
    }
    if has("--tcp-auto-tuning-status") {
        print(network::ensure_auto(&mut network::Windows, apply)?);
        return Ok(());
    }
    if has("--nic-rss-status") {
        print(nic::status()?);
        return Ok(());
    }
    if has("--nic-rss-apply") {
        print(nic::apply(apply)?);
        return Ok(());
    }
    if has("--nic-rss-restore") {
        print(nic::restore(apply)?);
        return Ok(());
    }
    if has("--memory-status") {
        let current = memory::snapshot()?;
        print(memory::Cleaner::new(&config["memoryCleaner"], current.total).status(current));
        return Ok(());
    }
    if has("--self-test") {
        let pid = std::process::id();
        let mask = process::affinity(pid)?;
        process::set_affinity(pid, mask)?;
        print(json!({"passed":true,"mask":format!("0x{mask:x}"),"path":process::path(pid)?}));
        return Ok(());
    }
    let mut affinity = Manager::new(
        config.clone(),
        root.join("runtime-state.json"),
        apply,
        true,
        None,
        has("--last-core"),
        has("--passive"),
    )?;
    if has("--reset-all") {
        print(affinity.reset_all()?);
        return Ok(());
    }
    if unsafe {
        windows_sys::Win32::System::Console::SetConsoleCtrlHandler(Some(console_signal), 1)
    } == 0
    {
        return Err(std::io::Error::last_os_error().to_string());
    }
    let initial = memory::snapshot()?;
    let mut cleaner = memory::Cleaner::new(&config["memoryCleaner"], initial.total);
    if !has("--quiet") {
        print(
            json!({"applyChanges":apply,"backgroundCpuRange":nogirem_backend::topology::cpu_range(&affinity.allocation.background_cpu_indexes),"gameCpuRange":nogirem_backend::topology::cpu_range(&affinity.allocation.game_cpu_indexes)}),
        );
    }
    let result = (|| {
        let auto = network::ensure_auto(&mut network::Windows, apply)?;
        let fast = network::ensure_fast(&mut network::Windows, apply, true)?;
        if !has("--quiet") {
            print(auto);
            print(fast);
        }
        affinity.recover()?;
        let affinity_interval =
            Duration::from_millis(config["pollIntervalMs"].as_u64().unwrap_or(1000).max(10));
        let memory_interval = Duration::from_millis(
            config["memoryCleaner"]["pollIntervalMs"]
                .as_u64()
                .unwrap_or(1000)
                .max(10),
        );
        let mut next_affinity = Instant::now();
        let mut next_memory = Instant::now() + memory_interval;
        while !STOP.load(Ordering::SeqCst) {
            let now = Instant::now();
            if now >= next_affinity {
                let before = affinity.game_active;
                if let Err(error) = affinity.tick() {
                    eprintln!("[poll-error] {error}");
                }
                if before != affinity.game_active {
                    println!(
                        "마비노기 프레임 부스트 {}",
                        if affinity.game_active { "On" } else { "Off" }
                    );
                    if before {
                        cleaner.armed = true;
                    }
                }
                next_affinity = now + affinity_interval;
            }
            if has("--memory-cleaner") && now >= next_memory {
                match memory::snapshot() {
                    Ok(current)
                        if cleaner.should_purge(
                            current,
                            nogirem_backend::now_ms(),
                            affinity.game_active,
                        ) =>
                    {
                        if apply {
                            cleaner.purge();
                        } else {
                            eprintln!("[dry-run] standby memory purge");
                            cleaner.armed = false;
                            cleaner.last_cleanup = nogirem_backend::now_ms();
                        }
                    }
                    Err(e) => eprintln!("[memory-error] {e}"),
                    _ => (),
                }
                next_memory = now + memory_interval;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        Ok(())
    })();
    let restored = affinity.restore_all();
    unsafe {
        windows_sys::Win32::System::Console::SetConsoleCtrlHandler(Some(console_signal), 0);
    }
    result.and(restored)
}
fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
