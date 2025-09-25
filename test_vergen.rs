fn main() {
    println!("VERGEN_GIT_SHA: {:?}", option_env!("VERGEN_GIT_SHA"));
    println!("VERGEN_GIT_DESCRIBE: {:?}", option_env!("VERGEN_GIT_DESCRIBE"));
    println!("VERGEN_GIT_BRANCH: {:?}", option_env!("VERGEN_GIT_BRANCH"));
    println!("VERGEN_GIT_COMMIT_TIMESTAMP: {:?}", option_env!("VERGEN_GIT_COMMIT_TIMESTAMP"));
}

