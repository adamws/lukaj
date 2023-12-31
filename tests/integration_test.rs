use dssim_core::{Dssim, DssimImage, Val};
use image::imageops::FilterType;
use image::DynamicImage;
use rgb::*;
use rstest::rstest;
use std::env;
use std::io;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::str;

const BASE: &'static str = env!("CARGO_MANIFEST_DIR");
const EXECUTABLE: &'static str = env!("CARGO_BIN_EXE_lukaj");
const TMPDIR: &'static str = env!("CARGO_TARGET_TMPDIR");

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

fn image(path: &String) -> Result<DynamicImage, String> {
    let image = image::open(path).map_err(|e| e.to_string())?;
    Ok(image)
}

fn dssim_image(img: DynamicImage, dssim: &Dssim) -> Result<DssimImage<f32>, String> {
    let width = usize::try_from(img.width()).map_err(|e| e.to_string())?;
    let height = usize::try_from(img.height()).map_err(|e| e.to_string())?;

    let raw = img.as_bytes().as_rgb();
    match dssim.create_image_rgb(raw, width, height) {
        Some(img) => Ok(img),
        None => Err(format!("Failed to create DSSIM image")),
    }
}

fn percentage_diff(u1: u32, u2: u32) -> f64 {
    let v1 = u1 as f64;
    let v2 = u2 as f64;
    (v1 - v2).abs() / ((v1 + v2) / 2.0) * 100.0
}

fn compare_images(actual: &String, expected: &String) -> Result<Val, String> {
    let dssim = Dssim::new();

    let img1 = image(actual)?;
    let mut img2 = image(expected)?;

    if img2.width() != img1.width() || img2.height() != img1.height() {
        // due to SVG backends differences, resulting image size might be slightly
        // different than reference (reference generated only for one backend)
        // let this slide if difference is less than 0.5% and force resize of result
        // (because same size images are required by dssim comparator)
        let width_diff = percentage_diff(img2.width(), img1.width());
        let height_diff = percentage_diff(img2.height(), img1.height());
        if width_diff < 0.5 && height_diff < 0.5 {
            img2 = img2.resize_exact(img1.width(), img1.height(), FilterType::Nearest);
        } else {
            return Err(format!("Images size difference exceeds acceptable limit"));
        }
    }

    let dssim1 = dssim_image(img1, &dssim)?;
    let dssim2 = dssim_image(img2, &dssim)?;

    Ok(dssim.compare(&dssim1, dssim2).0)
}

#[rstest]
#[cfg_attr(feature = "use-rsvg", case("rsvg-with-cairo"))]
#[cfg_attr(feature = "use-usvg", case("usvg-with-skia"))]
fn run_diff(
    #[case] backend: String,
    #[values((2, "arcs01", "arcs01_2"), (3, "tinycircle01", "tinycircle01"))] args: (
        u32,
        &str,
        &str,
    ),
) -> Result<(), String> {
    let scale = args.0;
    let files = (args.1, args.2);
    let screenshot_name = format!("{}-{}-{}.bmp", backend, files.0, files.1);
    let result = format!("{}/{}", TMPDIR, screenshot_name);
    let reference = format!(
        "{}/tests/references/run-diff-{}-{}.bmp",
        BASE, files.0, files.1
    );
    let threshold = 0.07;

    let mut command = Command::new(EXECUTABLE);
    command
        .env("CARGO_TARGET_TMPDIR", TMPDIR)
        .env("TEST_OUTPUT_FILENAME", screenshot_name)
        .args(&[
            &format!("-s{}", scale),
            "--backend",
            &backend,
            &format!("tests/images/{}.svg", files.0),
            &format!("tests/images/{}.svg", files.1),
        ]);

    let wrapped = wrap_with_xvfb(&mut command).map_err(|e| e.to_string())?;

    let output = wrapped.wait_with_output().map_err(|e| e.to_string())?;
    assert!(output.status.success());
    assert!(Path::new(&result).exists());

    let difference = compare_images(&result, &reference)?;
    assert!(
        difference < threshold,
        "Diffirence metric {} above threshold {}",
        difference,
        threshold
    );
    Ok(())
}

#[rstest]
#[cfg_attr(feature = "use-rsvg", case("rsvg-with-cairo"))]
#[cfg_attr(feature = "use-usvg", case("usvg-with-skia"))]
fn run_invalid_scale(
    #[case] backend: String,
    #[values(1, 80000)] scale: u32,
) -> Result<(), String> {
    let screenshot_name = "should_not_be_created.bmp";
    let result = format!("{}/{}", TMPDIR, screenshot_name);

    let mut command = Command::new(EXECUTABLE);
    command
        .env("CARGO_TARGET_TMPDIR", TMPDIR)
        .env("TEST_OUTPUT_FILENAME", screenshot_name)
        .args(&[
            &format!("-s{}", scale),
            "--backend",
            &backend,
            "tests/images/tinycircle01.svg",
            "tests/images/tinycircle01.svg",
        ]);

    let wrapped = wrap_with_xvfb(&mut command).map_err(|e| e.to_string())?;
    let output = wrapped.wait_with_output().map_err(|e| e.to_string())?;

    assert!(!output.status.success());
    assert!(!Path::new(&result).exists());

    Ok(())
}
