use std::str::FromStr;
fn main() {
    let src_dir = "workflows";
    let dest_dir = "../.github/workflows";

    for x in std::fs::read_dir(src_dir).unwrap() {
        let src_path = x.unwrap().path();
        println!("{}", src_path.display());
        let content = std::fs::read_to_string(&src_path).unwrap();
        let docs = yaml_rust::YamlLoader::load_from_str(&content).unwrap();
        let mut out_str = String::new();
        {
            let mut emitter = yaml_rust::YamlEmitter::new(&mut out_str);
            emitter.dump(&docs[0]).unwrap(); // dump the YAML object to a String
        }
        let dest_dir = std::path::PathBuf::from_str(dest_dir).unwrap();
        let destfile = dest_dir.join(src_path.file_name().unwrap());
        println!(
            "generate {} from {}",
            destfile.display(),
            src_path.display()
        );
        std::fs::write(&destfile, &out_str).unwrap();
    }
}
