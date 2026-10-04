//! `qinglink-service` 的构建脚本：把 `qinglink-service.rc`（应用清单与版本信息）
//! 内嵌进二进制，并在该资源文件发生变化时重新运行。

fn main() {
    println!("cargo:rerun-if-changed=qinglink-service.rc");
    #[cfg(windows)]
    embed_resource::compile("qinglink-service.rc", embed_resource::NONE)
        .manifest_optional()
        .expect("嵌入 qinglink-service.rc 失败");
}
