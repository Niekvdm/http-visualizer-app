fn main() {
    // Rerun if frontend changes
    println!("cargo:rerun-if-changed=frontend/");

    // Check if frontend directory exists
    let frontend_path = std::path::Path::new("frontend");
    if !frontend_path.exists() {
        eprintln!("Warning: frontend/ directory not found.");
        eprintln!("Run 'cd ../http-visualizer && yarn build' and copy dist/* to frontend/");

        // Create placeholder index.html for development
        std::fs::create_dir_all("frontend").ok();
        std::fs::write(
            "frontend/index.html",
            r#"<!DOCTYPE html>
<html>
<head>
    <title>HTTP Visualizer</title>
    <style>
        body { font-family: system-ui; display: flex; justify-content: center; align-items: center; height: 100vh; margin: 0; background: #1a1a2e; color: #eee; }
        .message { text-align: center; }
        code { background: #333; padding: 2px 8px; border-radius: 4px; }
    </style>
</head>
<body>
    <div class="message">
        <h1>HTTP Visualizer Backend</h1>
        <p>API is running. Frontend not embedded.</p>
        <p>To embed frontend:</p>
        <ol style="text-align: left;">
            <li>Build frontend: <code>cd http-visualizer && yarn build</code></li>
            <li>Copy to backend: <code>cp -r dist/* ../http-visualizer-app/frontend/</code></li>
            <li>Rebuild: <code>cargo build --release</code></li>
        </ol>
    </div>
</body>
</html>"#,
        ).ok();
    }
}
