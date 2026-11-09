//! Warm-process presentation + engine service workload, not a product SLO or
//! cold-start/terminal/network/I/O measurement. No document data is emitted.
#[cfg(target_os = "macos")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
        time::Instant,
    };
    use vellum_editor::{Action, Editor, reference_profile};
    use vellum_engine::instrumentation::NativeTimingProfileV1;
    let native = match std::env::args().skip(1).collect::<Vec<_>>().as_slice() {
        [] => false,
        [flag] if flag == "--native-unicode" => true,
        _ => {
            return Err(
                "usage: interaction [--native-unicode] (requires native-unicode feature)".into(),
            );
        }
    };
    #[cfg(not(feature = "native-unicode"))]
    if native {
        return Err("rebuild with --features native-unicode; no legacy fallback".into());
    }
    if native {
        eprintln!(
            "profile=native-unicode18; provider + engine + frames; excludes terminal drawing, cold startup and filesystem writing"
        );
    }
    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Fixture(PathBuf);
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_file(self.0.join("source"));
            let _ = fs::remove_dir(&self.0);
        }
    }
    println!(
        "bytes,capture,samples,p50_ns,p95_ns,p99_ns,max_ns,capture_drops_before_close,trace_window_overwrites_before_close"
    );
    for bytes in [1024, 64000] {
        for capacity in [0, 256, 1] {
            let directory = loop {
                let path = std::env::temp_dir().join(format!(
                    "vellum-ui-perf-{}-{}",
                    std::process::id(),
                    NEXT.fetch_add(1, Ordering::Relaxed)
                ));
                match fs::create_dir(&path) {
                    Ok(()) => break Fixture(path),
                    Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                    Err(e) => return Err(e.into()),
                }
            };
            let path = directory.0.join("source");
            let original = "prose\tcode\r\n".repeat(bytes / 12);
            fs::write(&path, &original)?;
            let mut config = reference_profile();
            config.capture_capacity = capacity;
            config.timing = if capacity == 0 {
                NativeTimingProfileV1::Off
            } else {
                NativeTimingProfileV1::ProcessMonotonicV1
            };
            let mut editor = Editor::open(std::slice::from_ref(&path), config)?;
            #[cfg(feature = "native-unicode")]
            if native {
                let mut profile = vellum_editor::NativeUnicodeProfile::reference();
                profile.capture_capacity = capacity;
                profile.timing = if capacity == 0 {
                    vellum_unicode_icu::Timing::Off
                } else {
                    vellum_unicode_icu::Timing::ProcessService
                };
                editor.enable_native_unicode(profile)?;
            }
            let mut samples = Vec::with_capacity(64);
            for iteration in 0..72 {
                let started = Instant::now();
                editor.apply(Action::Insert("x".into()))?;
                let typed = editor.frame(78, 20)?;
                if !typed.dirty
                    || !typed
                        .lines
                        .first()
                        .is_some_and(|line| line.starts_with('x'))
                {
                    return Err("typed projection oracle failed".into());
                }
                editor.apply(Action::Backspace)?;
                let restored = editor.frame(78, 20)?;
                let elapsed = started.elapsed().as_nanos();
                if restored.dirty || editor.text()? != original {
                    return Err("restoration oracle failed".into());
                }
                if iteration >= 8 {
                    samples.push(elapsed);
                }
            }
            samples.sort_unstable();
            let drops = editor.dropped_diagnostics();
            let overwritten = editor
                .diagnostic_window_overwrites()
                .map_or_else(|| "unknown".into(), |n| n.to_string());
            if (capacity == 1) != (drops > 0) {
                return Err("unexpected diagnostic loss population".into());
            }
            editor.close(false)?;
            #[cfg(feature = "native-unicode")]
            if native
                && (editor
                    .unicode_provider_usage()
                    .is_none_or(|u| u.completed != 144)
                    || editor.native_boundary_usage().retained_bytes != 0)
            {
                return Err("native version-cache/cleanup oracle failed".into());
            }
            if !editor.is_closed() || fs::read_to_string(&path)? != original {
                return Err("cleanup/disk oracle failed".into());
            }
            println!(
                "{},{capacity},64,{},{},{},{},{drops},{overwritten}",
                original.len(),
                samples[31],
                samples[60],
                samples[63],
                samples[63]
            );
        }
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("interaction workload currently requires macOS");
    std::process::exit(1);
}
