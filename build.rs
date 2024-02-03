

fn main() {
    println!("cargo:rustc-link-search=native=./libs");
    println!("cargo:rustc-link-lib=static=convert");
    println!("cargo:rustc-link-lib=static=avformat");
    println!("cargo:rustc-link-lib=static=avcodec");
    println!("cargo:rustc-link-lib=static=avutil");
}

