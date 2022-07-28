mod notechart_cache;

fn main() {
    let now = std::time::Instant::now();
    let directories: Vec<&str> = Vec::from(
        [
            "/home/righeil/Songs/ttt"
            //"/home/righeil/Songs/BMS Insane",
            //"/home/righeil/Games/soundsphere/userdata/charts/osu",
            //"/home/righeil/Games/soundsphere/userdata/charts/bms"
        ]
    );

    let res = notechart_cache::update(&directories);

    if let Err(e) = res {
        println!("{e}")
    }

    println!("{}", now.elapsed().as_secs_f32())
}
