use vergen_git2::{Emitter, Git2};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // set VERGEN_GIT_SHA env variable based on git commit hash
    let git2 = Git2::builder().sha(true).build();
    Emitter::default().add_instructions(&git2)?.emit()?;

    println!("cargo:rerun-if-changed=../../migrations");
    Ok(())
}
