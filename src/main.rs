use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::fs::File;
use std::env;
use clap::*;

arg_enum!{
    #[allow(non_camel_case_types)]
    #[derive(Debug)]
    enum Platform {
        x64,
        ARM,
        x86,
        ARM64,
    }
}

fn main() {
    let arg_matches = App::new("nuget_cpp")
                          .version(crate_version!())
                          .author("Robert Mikhayelyan <rob.mikh@outlook.com>")
                          .about("A tool that assists in packaging C++/WinRT components for NuGet.")
                          .arg(Arg::with_name("dir")
                                   .help("Sets the current directory when running the tool")
                                   .short("d")
                                   .long("dir")
                                   .takes_value(true))
                          .arg(Arg::with_name("all")
                                   .help("Restores project, builds all platforms, and packs [overrides everything]")
                                   .short("a")
                                   .long("all"))
                          .arg(Arg::with_name("restore")
                                   .help("Calls nuget restore")
                                   .short("r")
                                   .long("restore"))
                          .arg(Arg::with_name("build")
                                   .help("Builds the solution for Release")
                                   .short("b")
                                   .long("build")
                                   .possible_values(&Platform::variants())
                                   .multiple(true)
                                   .takes_value(true))
                          .arg(Arg::with_name("pack")
                                   .help("Packs the resulting files. Uses the nuget\\ directory")
                                   .short("p")
                                   .long("pack"))
                          .get_matches();

    let all = arg_matches.is_present("all");
    let restore = all || arg_matches.is_present("restore");
    let pack = all || arg_matches.is_present("pack");
    
    let mut build = false;
    let mut platforms : Vec<&str> = Vec::new();
    if all {
        build = true;

        let temp = ["x64", "ARM", "x86", "ARM64"];
        for platform in temp.iter() {
            platforms.push(platform);
        }
    } else if arg_matches.is_present("build") {
        build = true;
        let build_values = arg_matches.values_of("build").unwrap();
        for build_value in build_values {
            platforms.push(build_value);
        }
    }

    if arg_matches.is_present("dir") {
        let current_dir = arg_matches.value_of("dir").unwrap();
        println!("Using {}...", current_dir);
        env::set_current_dir(current_dir).expect("Failed to set the current directory.");
    }

    if restore {
        nuget_restore();
    }
    
    if build {
        for platform in platforms {
            msbuild_release(platform);
        }
    }
    
    if pack {
        nuget_pack();
    }
}

fn nuget_restore() {
    let status = Command::new("nuget")
                         .arg("restore")
                         .status()
                         .expect("process failed to execute");
    
    if !status.success() {
        panic!("nuget restore failed!");
    }
}

fn get_files_with_extension<P: AsRef<Path>>(folder_path: P, ext: &str) -> Option<Vec<PathBuf>> {
    let folder_path = folder_path.as_ref();
    let file_paths = std::fs::read_dir(folder_path).ok()?;
    let mut paths = Vec::new();
    for entry in file_paths {
        if let Ok(entry) = entry {
            let file_path = entry.path();
            if let Some(file_ext) = file_path.extension() {
                if file_ext == ext {
                    paths.push(file_path);
                }
            }
        }
    }
    Some(paths)
}

/*
msbuild <Solution>.sln /property:Configuration=Release /property:Platform=x64
*/
fn msbuild_release(plat: &str) {
    let solution = get_local_solution();

    let status = Command::new("msbuild")
                         .arg(solution)
                         .arg("/property:Configuration=Release")
                         .arg(format!("/property:Platform={}", plat))
                         .status()
                         .expect("process failed to execute");
    if !status.success() {
        panic!("msbuild for {} failed!", plat);
    }
}

fn get_local_solution() -> std::path::PathBuf {
    let current_dir = std::env::current_dir().expect("Failed to query current directory.");
    let mut solution_paths = get_files_with_extension(current_dir, "sln").expect("Failed to search files.");

    if solution_paths.is_empty() {
        panic!("No solution files found!");
    } else if solution_paths.len() > 1 {
        panic!("Too many solution files found!");
    }

    solution_paths.pop().unwrap()
}

fn nuget_pack_directory<P: AsRef<Path>>(nuget_path: P) {
    let nuspec = get_nugetpkg_nuspec(&nuget_path);
    let version = get_nugetpkg_version(nuget_path);

    let status = Command::new("nuget")
                         .arg("pack")
                         .arg(nuspec)
                         .args(&["-version", &version])
                         .status()
                         .expect("process failed to execute");
    if !status.success() {
        panic!("nuget pack failed!");
    }
}

fn nuget_pack_projects() {
    // Search for nuget directories in project folders
    // TODO: Read projects from solution file
    let current_dir = std::env::current_dir().expect("Failed to query current directory.");
    let entries = std::fs::read_dir(current_dir).expect("Failed to open current directory.");
    for entry in entries {
        if let Ok(entry) = entry {
            let metadata = entry.metadata().unwrap();
            if metadata.is_dir() {
                let nuget_path = {
                    let mut nuget_path = entry.path();
                    nuget_path.push("nuget");
                    nuget_path
                };
                if nuget_path.exists() {
                    nuget_pack_directory(nuget_path);
                }
            }
        }
    }
}

fn nuget_pack() {
    let nuget_path = Path::new("nuget");
    if nuget_path.exists() {
        nuget_pack_directory(nuget_path);
    } else {
        nuget_pack_projects();
    }
}

fn get_nugetpkg_nuspec<P: AsRef<Path>>(nuget_path: P) -> std::path::PathBuf {
    let mut nuspec_paths = get_files_with_extension(nuget_path, "nuspec").expect("Failed to look for solution files.");

    if nuspec_paths.is_empty() {
        panic!("No nuspec files found!");
    } else if nuspec_paths.len() > 1 {
        panic!("Too many nuspec files found!");
    }

    nuspec_paths.pop().unwrap()
}

fn get_nugetpkg_version<P: AsRef<Path>>(nuget_path: P) -> String {
    let version_path = {
        let mut version_path = nuget_path.as_ref().to_owned();
        version_path.push("VERSION");
        version_path
    };
    let mut version_file = File::open(version_path)
                                .expect("Failed to open VERSION file");

    let mut version_string = String::new();
    version_file.read_to_string(&mut version_string).unwrap();

    version_string
}
