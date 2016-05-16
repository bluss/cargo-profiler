#[macro_use]
extern crate lazy_static;
extern crate clap;
extern crate regex;

pub mod profiler;
pub mod parse;
pub mod display;
pub mod err;
pub mod argparse;
pub mod cargo;

use clap::{Arg, App, SubCommand, AppSettings};
use profiler::Profiler;
use parse::callgrind::CallGrindParser;
use parse::cachegrind::CacheGrindParser;
use std::process::Command;
use err::ProfError;
use argparse::{get_profiler, get_binary, get_num, get_sort_metric};
use cargo::{find_toml, get_package_name, build_binary};

fn main() {
    let _ = real_main();
}

// #[cfg(all(unix, any(target_os = "linux", target_os = "macos")))]
#[cfg(unix)]
fn real_main() -> Result<(), ProfError> {
    // create binary path argument
    let binary_arg = Arg::with_name("binary")
                         .long("bin")
                         .value_name("BINARY")
                         .required(false)
                         .help("binary you want to profile");
    // create release argument

    let release = Arg::with_name("release")
                      .long("release")
                      .required(false)
                      .help("whether binary should be built in release mode");

    // create function count argument
    let fn_count_arg = Arg::with_name("n")
                           .short("n")
                           .value_name("NUMBER")
                           .takes_value(true)
                           .help("number of functions you want");

    // create sort metric argument
    let sort_arg = Arg::with_name("sort")
                       .long("sort")
                       .value_name("SORT")
                       .takes_value(true)
                       .help("metric you want to sort by");

    // create callgrind subcommand
    let callgrind = SubCommand::with_name("callgrind")
                        .about("gets callgrind features")
                        .version("1.0")
                        .author("Suchin Gururangan")
                        .arg(release.clone())
                        .arg(binary_arg.clone())
                        .arg(fn_count_arg.clone());

    // create cachegrind subcommand
    let cachegrind = SubCommand::with_name("cachegrind")
                         .about("gets cachegrind features")
                         .version("1.0")
                         .author("Suchin Gururangan")
                         .arg(release)
                         .arg(binary_arg)
                         .arg(fn_count_arg)
                         .arg(sort_arg);

    // create profiler subcommand
    let profiler = SubCommand::with_name("profiler")
                       .about("gets callgrind features")
                       .version("1.0")
                       .author("Suchin Gururangan")
                       .subcommand(callgrind)
                       .subcommand(cachegrind);

    // create profiler application
    let matches = App::new("cargo-profiler")
                      .bin_name("cargo")
                      .settings(&[AppSettings::SubcommandRequired])
                      .version("1.0")
                      .author("Suchin Gururangan")
                      .about("Profile your binaries")
                      .subcommand(profiler)
                      .get_matches();

    // parse arguments from cli call
    let (m, profiler) = try!(get_profiler(&matches));
    let toml_dir = find_toml().unwrap();
    let package_name = get_package_name(&toml_dir).ok().unwrap();
    let binary = {
        if m.is_present("binary") {
            try!(get_binary(&m)).to_string()
        } else {
            if m.is_present("release") {
                try!(build_binary(true, &package_name[..]))
            } else {
                try!(build_binary(false, &package_name[..]))
            }
        }
    };

    let num = try!(get_num(&m));
    let sort_metric = try!(get_sort_metric(&m));

    match profiler {
        Profiler::CallGrind { .. } => {
            println!("\n\x1b[1;33mProfiling \x1b[1;0m{} \x1b[0mwith callgrind\x1b[0m...",
                     &package_name[..])
        }
        Profiler::CacheGrind { .. } => {
            println!("\n\x1b[1;33mProfiling \x1b[1;0m{} \x1b[0mwith cachegrind\x1b[0m...",
                     &package_name[..])
        }
    };

    // get the profiler output
    let output = match profiler {
        Profiler::CallGrind { .. } => try!(profiler.callgrind_cli(&binary)),
        Profiler::CacheGrind { .. } => try!(profiler.cachegrind_cli(&binary)),
    };
    // parse the output into struct
    let parsed = match profiler {
        Profiler::CallGrind { .. } => try!(profiler.callgrind_parse(&output, num)),
        Profiler::CacheGrind { .. } => try!(profiler.cachegrind_parse(&output, num, sort_metric)),
    };

    // pretty-print
    println!("{}", parsed);

    // remove files generated while profiling
    try!(Command::new("rm")
             .arg("cachegrind.out")
             .output());


    try!(Command::new("rm")
             .arg("callgrind.out")
             .output());

    Ok(())
}
