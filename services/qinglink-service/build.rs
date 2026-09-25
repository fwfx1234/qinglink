fn main() {
    println!("cargo:rerun-if-changed=qinglink-service.rc");
    #[cfg(windows)]
    embed_resource::compile("qinglink-service.rc", embed_resource::NONE)
        .manifest_optional()
        .unwrap();
}
