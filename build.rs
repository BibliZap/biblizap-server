use static_files::resource_dir;

fn main() -> std::io::Result<()> {
    change_detection::ChangeDetection::path("./frontend/dist").generate();
    resource_dir("./frontend/dist").build()
}
