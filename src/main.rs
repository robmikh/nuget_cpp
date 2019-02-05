extern crate glob;
#[macro_use]
extern crate clap;

use std::io::prelude::*;
use std::process::Command;
use std::fs::File;
use std::env;
use clap::*;

arg_enum!{
    #[derive(Debug)]
    enum Platform {
        x64,
        ARM,
        x86
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

        let temp = ["x64", "ARM", "x86"];
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
        env::set_current_dir(current_dir).expect("failed to set current dir");
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
    let mut solution_paths = glob::glob("*.sln")
                                  .expect("failed to look for solutions")
                                  .collect::<Vec<_>>();

    if solution_paths.len() <= 0 {
        panic!("No solution files found!");
    } else if solution_paths.len() > 1 {
        panic!("Too many solution files found!");
    }

    solution_paths.pop().unwrap().unwrap()
}

fn nuget_pack() {
    let nuspec = get_nugetpkg_nuspec();
    let version = get_nugetpkg_version();

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

fn get_nugetpkg_nuspec() -> std::path::PathBuf {
    let mut nuspec_paths = glob::glob("nuget\\*.nuspec")
                                .expect("failed to look for solutions")
                                .collect::<Vec<_>>();

    if nuspec_paths.len() <= 0 {
        panic!("No nuspec files found!");
    } else if nuspec_paths.len() > 1 {
        panic!("Too many nuspec files found!");
    }

    nuspec_paths.pop().unwrap().unwrap()
}

fn get_nugetpkg_version() -> String {
    let mut version_file = File::open("nuget\\VERSION")
                                .unwrap();

    let mut version_string = String::new();
    version_file.read_to_string(&mut version_string).unwrap();

    version_string
}
