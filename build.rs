use static_files::resource_dir;

fn main() -> std::io::Result<()> {
    change_detection::ChangeDetection::path("./biblizap-frontend/dist").generate();
    resource_dir("./biblizap-frontend/dist").build()
}

