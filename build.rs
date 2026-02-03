use std::process::Command;

fn main() {
    // Rebuild if frontend files change
    println!("cargo:rerun-if-changed=frontend/input.css");
    println!("cargo:rerun-if-changed=frontend/index.html");
    println!("cargo:rerun-if-changed=frontend/login.html");
    println!("cargo:rerun-if-changed=frontend/app.js");

    // Create dist directory if it doesn't exist
    std::fs::create_dir_all("frontend/dist").expect("Failed to create frontend/dist directory");

    // Run tailwindcss
    let status = Command::new("tailwindcss")
        .args([
            "--input",
            "frontend/input.css",
            "--output",
            "frontend/dist/output.css",
            "--minify",
        ])
        .status()
        .expect("Failed to run tailwindcss. Make sure it is installed: npm install -g @tailwindcss/cli");

    if !status.success() {
        panic!("TailwindCSS failed with exit code: {:?}", status.code());
    }
}
