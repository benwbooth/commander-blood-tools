//! Dump each script's BAS concept-menu stack: the topic labels the engine derives.

use commander_blood_tools::engine::EngineState;
use std::path::Path;

fn main() {
    let iso = Path::new("output/_tmp_iso");
    for n in 1..=5u32 {
        let rd = |ext: &str| std::fs::read(iso.join(format!("SCRIPT{n}.{ext}")));
        let (Ok(bas), Ok(dic)) = (rd("BAS"), rd("DIC")) else {
            println!("SCRIPT{n}: missing BAS/DIC");
            continue;
        };
        let mut e = EngineState::new();
        e.load_bas_menus(&bas, &dic);
        e.sync_topic_menu_from_bas();
        println!("SCRIPT{n}: entry menu = {:?}", e.current_bas_menu_labels());
    }
}
