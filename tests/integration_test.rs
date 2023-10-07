use rstest::rstest;
use std::env;
use std::io;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::str;

fn wrap_with_xvfb(command: &mut Command) -> io::Result<Child> {
    let mut check_command = Command::new("xvfb-run");
    check_command
        .arg("-h")
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    match check_command.status() {
        Ok(exit_status) if exit_status.success() => {
            let mut wrapper_command = Command::new("xvfb-run");
            // not all xvfb-run implementations include '-d' option (ubuntu for example)
            // recommended to use archlinux version (see docker/Dockerfile for details)
            wrapper_command
                .args(&["-d", "-s", "-screen 0 3840x2160x24"])
                .arg(command.get_program())
                .args(command.get_args());

            for env in command.get_envs() {
                match env {
                    (k, Some(v)) => wrapper_command.env(k, v),
                    (k, None) => wrapper_command.env_remove(k),
                };
            }
            wrapper_command.spawn()
        }
        _ => command.spawn(),
    }
}

fn compare_images(actual: &String, expected: &String) -> f64 {
    let output = Command::new("magick")
        .arg("compare")
        .arg(actual)
        .arg(expected)
        .args(&["-metric", "DSSIM", "null:"])
        .output()
        .expect("failed to execute process");

    String::from_utf8(output.stderr)
        .unwrap()
        .parse::<f64>()
        .unwrap()
}

#[rstest]
#[case("rsvg-with-cairo")]
#[case("usvg-with-skia")]
fn run_diff(#[case] backend: String) {
    let base: &'static str = env!("CARGO_MANIFEST_DIR");
    let executable: &'static str = env!("CARGO_BIN_EXE_lukaj");
    let tmpdir: &'static str = env!("CARGO_TARGET_TMPDIR");

    let screenshot_name = format!("{}.bmp", backend);
    let result = format!("{}/{}", tmpdir, screenshot_name);
    let reference = format!("{}/tests/references/run_diff.bmp", base);
    let threshold = 0.1;

    let mut command = Command::new(executable);
    command
        .env("CARGO_TARGET_TMPDIR", tmpdir)
        .env("TEST_OUTPUT_FILENAME", screenshot_name)
        .args(&[
            "-s5",
            "--backend",
            &backend,
            "tests/images/arcs01.svg",
            "tests/images/arcs01_2.svg",
        ]);

    let wrapped = wrap_with_xvfb(&mut command).expect("Failed to create command");

    let output = wrapped.wait_with_output().unwrap();
    assert!(output.status.success());
    assert!(Path::new(&result).exists());

    let difference = compare_images(&result, &reference);
    assert!(
        difference < threshold,
        "Diffirence metric {} above threshold {}",
        difference,
        threshold
    );
}
