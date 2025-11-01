use anyhow::{Context, Result, anyhow};
use clap::Parser;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use regex::Regex;
use std::io::{self, Write};
use std::process::{Command, Stdio, Child};
use std::path::PathBuf;
use std::fs;
use std::thread;
use std::time::Duration;

#[derive(Parser)]
#[command(name = "igrok")]
#[command(author = "Mr_fox")]
#[command(version = "1.0")]
#[command(about = "play YouTube audio with real-time visualization", long_about = None)]
struct Cli {
    /// YouTube video or playlist URL
    #[arg(short, long)]
    url: Option<String>,

    /// Output directory for downloads
    #[arg(short, long, default_value = "~/Music/youtube-dl")]
    output: String,

    /// Skip visualization (Cava)
    #[arg(long)]
    no_viz: bool,
}

struct App {
    output_dir: PathBuf,
    no_viz: bool,
}

impl App {
    fn new(output: String, no_viz: bool) -> Result<Self> {
        let output_dir = shellexpand::tilde(&output).to_string();
        let output_dir = PathBuf::from(output_dir);
        
        // Create output directory if it doesn't exist
        fs::create_dir_all(&output_dir)
            .context("Failed to create output directory")?;

        Ok(App {
            output_dir,
            no_viz,
        })
    }

    fn check_dependencies(&self) -> Result<()> {
        println!("{}", "Checking dependencies...".cyan().bold());
        
        let deps = vec![
            ("yt-dlp", "YouTube downloader"),
            ("mpv", "Media player"),
            ("cava", "Audio visualizer"),
        ];

        for (cmd, _desc) in deps {
            if cmd == "cava" && self.no_viz {
                continue;
            }
            
            match Command::new("which").arg(cmd).output() {
                Ok(output) if output.status.success() => {
                
                }
                _ => {
                    return Err(anyhow!(
                        "{} is not installed. Install it with: sudo pacman -S {}",
                        cmd, cmd
                    ));
                }
            }
        }
        
        println!();
        Ok(())
    }

    fn get_url(&self, url_arg: Option<String>) -> Result<String> {
        if let Some(url) = url_arg {
            return Ok(url);
        }

        print!("{}", " Enter  URL: ".yellow().bold());
        io::stdout().flush()?;
        
        let mut url = String::new();
        io::stdin().read_line(&mut url)?;
        let url = url.trim().to_string();
        
        if url.is_empty() {
            return Err(anyhow!("No URL provided"));
        }
        
        Ok(url)
    }

    fn validate_url(&self, url: &str) -> Result<()> {
        let youtube_regex = Regex::new(
            r"^(https?://)?(www\.)?(youtube\.com|youtu\.be|music\.youtube\.com)/.+"
        )?;
        
        if !youtube_regex.is_match(url) {
            return Err(anyhow!("Invalid YouTube URL"));
        }
        
        Ok(())
    }

    fn download_audio(&self, url: &str) -> Result<Vec<PathBuf>> {
        println!("{}", "\n Loading audio...".cyan().bold());
        
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap()
        );
        spinner.set_message("Fetching metadata...");
        
        let output_template = self.output_dir
            .join("%(title)s.%(ext)s")
            .to_string_lossy()
            .to_string();

        let status = Command::new("yt-dlp")
            .args([
                "-q",
                "-x",
                "--audio-format", "mp3",
                "--audio-quality", "0",
                "-o", &output_template,
                "--print", "after_move:filepath",
                url,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?
            .wait()?;

        spinner.finish_and_clear();

        if !status.success() {
            return Err(anyhow!("Loading failed"));
        }

        // Find downloaded files
        let files = self.find_recent_files()?;
        
        if files.is_empty() {
            return Err(anyhow!("No files were Loaded"));
        }

        println!("{} Downloaded {} file(s)", "".green(), files.len());
        Ok(files)
    }

    fn find_recent_files(&self) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        
        for entry in fs::read_dir(&self.output_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "mp3" || ext == "m4a" || ext == "opus" {
                        files.push(path);
                    }
                }
            }
        }
        
        files.sort_by_key(|p| {
            fs::metadata(p)
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
        });
        
        // Get most recent files
        files.reverse();
        Ok(files.into_iter().take(10).collect())
    }

    fn play_with_visualization(&self, files: Vec<PathBuf>) -> Result<()> {
        println!("{}", "\nStarting playback...".green().bold());
        
        for (idx, file) in files.iter().enumerate() {
            let filename = file.file_name()
                .unwrap_or_default()
                .to_string_lossy();
            
            println!("\n{} {} [{}/{}]", 
                "▶".green().bold(),
                filename.bright_white().bold(),
                idx + 1,
                files.len()
            );

            self.play_file(file)?;
        }

        Ok(())
    }

    fn play_file(&self, file: &PathBuf) -> Result<()> {
        // Start mpv
        let mut mpv = Command::new("mpv")
            .args([
                "--really-quiet",
                "--no-video",
                "--volume=100",
                file.to_str().unwrap(),
            ])
            .spawn()
            .context("Failed to start mpv")?;

        // Start cava if enabled
        let mut cava: Option<Child> = None;
        if !self.no_viz {
            thread::sleep(Duration::from_millis(500)); // Let mpv initialize
            
            match Command::new("cava").spawn() {
                Ok(child) => {
                    cava = Some(child);
                    println!("{}", "   Visualization active (press 'q' in mpv to stop)".dimmed());
                }
                Err(_) => {
                    println!("{}", "   Cava failed to start".yellow());
                }
            }
        }

        // Wait for mpv to finish
        let status = mpv.wait()?;

        // Stop cava
        if let Some(mut c) = cava {
            let _ = c.kill();
            let _ = c.wait();
        }

        if !status.success() {
            return Err(anyhow!("Playback interrupted"));
        }

        Ok(())
    }

    fn run(&self, url_arg: Option<String>) -> Result<()> {
        // Print banner
        self.print_banner();

        // Check dependencies
        self.check_dependencies()?;

        // Get URL
        let url = self.get_url(url_arg)?;
        
        // Validate URL
        self.validate_url(&url)?;
        println!("{} URL validated", "✓".green());

        // Download
        let files = self.download_audio(&url)?;

        // Play
        self.play_with_visualization(files)?;

        println!("\n{}", " Playback complete!".green().bold());
        Ok(())
    }

    fn print_banner(&self) {
        println!("\n{}", "╔════════════════════════════════════════╗".cyan());
        println!("{}", "║                Igrok                   ║".cyan());
        println!("{}", "╚════════════════════════════════════════╝".cyan());
    }
}

fn main() {
    let cli = Cli::parse();
    
    let app = match App::new(cli.output, cli.no_viz) {
        Ok(app) => app,
        Err(e) => {
            eprintln!("{} {}", "Error:".red().bold(), e);
            std::process::exit(1);
        }
    };

    if let Err(e) = app.run(cli.url) {
        eprintln!("\n{} {}", "Error:".red().bold(), e);
        std::process::exit(1);
    }
}
