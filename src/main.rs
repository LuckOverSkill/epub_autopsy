use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::env;
use anyhow::{Context, Result};
use roxmltree::Document;
use sanitize_filename::sanitize;
use dirs;

fn main() -> Result<()> {
    // --- SETUP ---
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: epub_autopsy <path_to_epub>");
        return Ok(());
    }
    // Define input_path:
    let input_path = Path::new(&args[1]); 

    // ----------------------------------------------------
    // START: CORRECTED PATH LOGIC
    // ----------------------------------------------------

    // 1. Get the base Documents path
    let documents_path = dirs::document_dir()
        .context("Could not find the user's Documents directory!")?;

    // 2. Define the FINAL, hardcoded location inside Documents
    let output_base = documents_path.join("Split_Books");

    // 3. Define and use book_name (used for the subfolder)
    let book_name = input_path.file_stem().unwrap().to_string_lossy();

    // 4. Construct the final output path
    let output_dir = output_base.join(&*book_name);

    // Create output directory
    if output_dir.exists() { fs::remove_dir_all(&output_dir)?; }
    fs::create_dir_all(&output_dir)?;

    println!("📖 Opening book: {}", book_name);
    
    // Open the ZIP
    let file = fs::File::open(input_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    // --- LOCATE OPF (METADATA) ---
    // 1. Find path in container.xml
    let opf_path = {
        let mut container = archive.by_name("META-INF/container.xml")
            .context("Not a valid EPUB (missing container.xml)")?;
        let mut content = String::new();
        container.read_to_string(&mut content)?;
        let doc = Document::parse(&content)?;
        doc.descendants()
            .find(|n| n.has_tag_name("rootfile"))
            .context("No rootfile")?
            .attribute("full-path")
            .context("No full-path")?
            .to_string()
    };

    // 2. Parse OPF to get the file list
    let opf_content = {
        let mut f = archive.by_name(&opf_path)?;
        let mut s = String::new();
        f.read_to_string(&mut s)?;
        s
    };
    let opf_doc = Document::parse(&opf_content)?;

    // 3. Map IDs to File Paths
    let manifest = opf_doc.descendants().find(|n| n.has_tag_name("manifest")).context("No manifest")?;
    let mut id_to_href = std::collections::HashMap::new();
    for item in manifest.children() {
        if let (Some(id), Some(href)) = (item.attribute("id"), item.attribute("href")) {
            id_to_href.insert(id, href);
        }
    }

    // 4. Get the Spine (Chapter Order)
    let spine = opf_doc.descendants().find(|n| n.has_tag_name("spine")).context("No spine")?;
    let spine_ids: Vec<&str> = spine.children()
        .filter(|n| n.is_element())
        .filter_map(|n| n.attribute("idref"))
        .collect();

    println!("🔪 Extracting {} chapters to Plain Text...", spine_ids.len());

    // --- EXTRACT AND CONVERT ---
    let opf_parent = Path::new(&opf_path).parent().unwrap_or(Path::new(""));

    for (index, idref) in spine_ids.iter().enumerate() {
        let href = match id_to_href.get(idref) {
            Some(h) => h,
            None => continue, // Skip if broken link
        };

        // Calculate path inside ZIP
        // OPF location + href location (e.g. OEBPS/ + chapter1.xhtml)
        let zip_path = opf_parent.join(href).to_string_lossy().replace("\\", "/");

        let output_filename = format!("{:02}_{}.txt", index + 1, sanitize(href));
        let output_file_path = output_dir.join(output_filename.replace(".xhtml", "").replace(".html", ""));

        // Read content from ZIP
let content_result = {
    match archive.by_name(&zip_path) {
        Ok(mut file) => {
            html2text::from_read(&mut file, 1000) 
        },
        Err(_) => continue, // File not found in zip, skip
    }
};

    // Check the actual length of the extracted text.
    // If it's shorter than 200 characters, skip it (it's likely a filler page).
    const MIN_CONTENT_LENGTH: usize = 200; 

    if content_result.len() < MIN_CONTENT_LENGTH {
     println!("   skipping: {:?} (Content too short)",  output_file_path.file_name().unwrap());
        continue; 
    }
    // --- END FILTERING LOGIC ---

    // Write to .txt file
    fs::write(&output_file_path, content_result)?;
    println!("   saved: {:?}", output_file_path.file_name().unwrap());
    }

    println!("\n✨ Done! Text files are in: {:?}", output_dir);
    Ok(())
}
