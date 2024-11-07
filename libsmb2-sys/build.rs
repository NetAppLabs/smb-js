extern crate bindgen;

use std::env;
use std::path::{Path, PathBuf};

fn main() {
    let link_static_env_if_any = env::var("LIBSMB_LINK_STATIC");
    match link_static_env_if_any {
        Ok(link_static) => {
            if link_static == "true" {
                println!("cargo:rustc-link-lib=static=smb2");
            } else {
                println!("cargo:rustc-link-lib=smb2"); 
            }
        },
        Err(_) => {
            println!("cargo:rustc-link-lib=smb2");
        },
    }
    let libsmb2_lib_path_if_any = env::var("LIBSMB_LIB_PATH");
    match libsmb2_lib_path_if_any {
        Ok(lib_dir) => {
            let lib_dir = Path::new(&lib_dir);
            println!("cargo:rustc-link-search=native={}", lib_dir.display());
        },
        Err(_) => {},
    }

    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let mut builder = bindgen::Builder::default()
    // The input header we would like to generate
    // bindings for.
    .header("wrapper.h");
    
    let libsmb2_include_path_if_any = env::var("LIBSMB_INCLUDE_PATH");
    match libsmb2_include_path_if_any {
        Ok(include_path) => {
            let include_path = Path::new(&include_path);
            builder = builder.clang_arg(format!("-I{}", include_path.display()));

        },
        Err(_) => {},
    }

    //.blacklist_type("statvfs")
    let bindings = builder
    // Finish the builder and generate the bindings.
    .generate()
    // Unwrap the Result and panic on failure.
    .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
