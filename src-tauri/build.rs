fn main() {
    // 设置 libclang 路径
    if cfg!(target_os = "windows") {
        let llvm_path = std::env::var("LIBCLANG_PATH").unwrap_or_else(|_| {
            // 默认路径
            "D:\\Dev\\LLVM\\bin".to_string()
        });
        println!("cargo:rustc-env=LIBCLANG_PATH={}", llvm_path);
    }
    
    tauri_build::build()
}
