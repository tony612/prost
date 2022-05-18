use prost_build::Config;

fn main() {
    Config::new()
        .out_dir("src/pb")
        .compile_protos(&["protos/addressbook.proto"], &["protos"])
        .unwrap();
}
