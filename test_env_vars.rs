fn main() {
    println!("VERGEN_GIT_SHA: {:?}", option_env!("VERGEN_GIT_SHA"));
    println!("VERGEN_GIT_DESCRIBE: {:?}", option_env!("VERGEN_GIT_DESCRIBE"));
    println!("VERGEN_GIT_BRANCH: {:?}", option_env!("VERGEN_GIT_BRANCH"));
    println!("VERGEN_GIT_COMMIT_TIMESTAMP: {:?}", option_env!("VERGEN_GIT_COMMIT_TIMESTAMP"));
    
    // Also check the old variables that were being used
    println!("GIT_TAG: {:?}", option_env!("GIT_TAG"));
    println!("GIT_COMMIT_FULL: {:?}", option_env!("GIT_COMMIT_FULL"));
}

