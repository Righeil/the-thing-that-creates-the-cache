mod notechart_cache;

struct NotechartSetDir {
    pub id: i64,
    pub path: String,
    pub file_paths: Vec<String>
}

fn main() {
    let now = std::time::Instant::now();
    let directories: Vec<&str> = Vec::from(
        [
            //"/home/righeil/Songs/ttt"
            "/home/righeil/Songs/BMS Insane",
            "/home/righeil/Games/soundsphere/userdata/charts/osu",
            "/home/righeil/Games/soundsphere/userdata/charts/bms"
        ]
    );

    notechart_cache::update(&directories).unwrap();

    println!("{}", now.elapsed().as_secs_f32())
}
