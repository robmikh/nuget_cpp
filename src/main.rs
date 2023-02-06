mod cli;

use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::fs::File;
use std::env;

use clap::Parser;
use cli::{Args, Platform};

fn main() {
    let args = Args::parse();

    let all = args.all;
    let restore = all || args.restore;
    let pack = all || args.pack;
    
    let (build, platforms) = {
        let mut platforms = Vec::with_capacity(args.build.len());
        for platform in &args.build {
            platforms.push(format!("{}", platform));
        }
        if all && platforms.is_empty() {
            let build_platforms = [Platform::x64, Platform::x86, Platform::ARM64, Platform::ARM];
            for platform in build_platforms {
                platforms.push(format!("{}", platform));
            }
        }
        (all || !args.build.is_empty(), platforms)
    };

    if let Some(current_dir) = args.dir.as_ref() {
        println!("Using {}...", current_dir);
        env::set_current_dir(current_dir).expect("Failed to set the current directory.");
    }

    // Find our projects
    let projects = {
        let mut projects = Vec::new();
        for project_dir in get_project_dirs_with_nuget_dirs() {
            let project_paths = get_files_with_extension(project_dir, "vcxproj").expect("Failed to search files.");

            if project_paths.is_empty() {
                panic!("No project files found!");
            } else if project_paths.len() > 1 {
                panic!("Too many project files found!");
            }

            projects.push(project_paths.first().unwrap().clone());
        }
        projects
    };

    if restore {
        println!("Restoring...");
        for project in &projects {
            nuget_restore(project);
        }
    }

    if build {
        println!("Building for these platforms: {:?}", &platforms);
        for project in &projects {
            for platform in &platforms {
                msbuild_release(project, &platform);
            }
        }
    }
    
    if pack {
        println!("Packing...");
        nuget_pack();
    }
}

fn nuget_restore<P: AsRef<Path>>(project_path: P) {
    let solution = get_local_solution();
    let solution_dir = solution.parent().unwrap();

    let status = Command::new("nuget")
                         .arg("restore")
                         .arg(project_path.as_ref())
                         .arg("-SolutionDirectory")
                         .arg(solution_dir)
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
msbuild <Solution>.sln /t:<Project>.vcxproj /property:Configuration=Release /property:Platform=x64
*/
fn msbuild_release<P: AsRef<Path>>(project_path: P, plat: &str) {
    let solution = get_local_solution();

    let project_path = project_path.as_ref();
    let project_path_stem = project_path.file_stem().unwrap();
    let project_path_name = project_path_stem.to_str().unwrap();
    // TODO: Filter other illegal characters
    let project_name = project_path_name.replace(".", "_");

    let status = Command::new("msbuild")
                         .arg(solution)
                         .arg(format!("/t:{}", project_name))
                         .arg("/property:Configuration=Release")
                         .arg(format!("/property:Platform={}", plat))
                         .status()
                         .expect("process failed to execute");
    if !status.success() {
        panic!("msbuild for {} failed!", plat);
    }
}

/*
msbuild <Solution>.sln /property:Configuration=Release /property:Platform=x64
*/
fn msbuild_release_solution(plat: &str) {
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

fn get_project_dirs_with_nuget_dirs() -> Vec<PathBuf> {
    // Search for nuget directories in project folders
    // TODO: Read projects from solution file
    let current_dir = std::env::current_dir().expect("Failed to query current directory.");
    let entries = std::fs::read_dir(current_dir).expect("Failed to open current directory.");
    let mut project_dirs = Vec::new();
    for entry in entries {
        if let Ok(entry) = entry {
            let metadata = entry.metadata().unwrap();
            if metadata.is_dir() {
                let entry_path = entry.path();
                let nuget_path = {
                    let mut nuget_path = entry_path.clone();
                    nuget_path.push("nuget");
                    nuget_path
                };
                if nuget_path.exists() {
                    project_dirs.push(entry_path);
                }
            }
        }
    }
    project_dirs
}

fn nuget_pack_projects() {
    let project_dirs = get_project_dirs_with_nuget_dirs();
    for project_dir in project_dirs {
        let mut nuget_path = project_dir;
        nuget_path.push("nuget");
        nuget_pack_directory(nuget_path);
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
