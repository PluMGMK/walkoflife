use std::{time,thread::sleep};
use walkoflife::{memory::read_prims,utils};

fn main() -> Result<(), String> {
    let r2pid = match utils::find_attach_rayman2() {
        Ok(ans) => ans,
        Err(errstr) => {
            return Err(format!("{} - is Rayman2.exe running?", errstr));
        }
    };

    let interval = time::Duration::from_millis(1000);
    loop {
        sleep(interval);
        // We only care about the Walk of Life
        if utils::get_current_level_name(r2pid)?.to_lowercase() != "ly_10" {
            break;
        }
        let object_types = utils::read_object_types(r2pid)?;
        let active_super_objects = utils::get_active_super_object_names(r2pid, &object_types[2], 0)?;
        let global_ptr = active_super_objects["global"];
        let timerobj_ptr = active_super_objects["GRP_TimerCourse_I3"];
        let timer_ptr = utils::get_dsg_var_ptr(r2pid, timerobj_ptr, 84)?; // Float_16
        let countdown_ptr = utils::get_dsg_var_ptr(r2pid, global_ptr, 84)?; // Int_30

        let timer: f32 = read_prims(r2pid, timer_ptr, 1).unwrap()[0];
        let countdown: i32 = read_prims(r2pid, countdown_ptr, 1).unwrap()[0];

        println!("{} -> {}", countdown, timer);

        // Try to figure out some other stuff…
        let framerate: f32 = read_prims(r2pid, 0x5036A8, 1).unwrap()[0];
        let inverse_framerate: f32 = read_prims(r2pid, 0x50043C, 1).unwrap()[0];
        let delta_t: i32 = read_prims(r2pid, 0x500434, 1).unwrap()[0];
        println!("Frame rate: {}; Inverse frame rate: {}; Delta t: {}", framerate, inverse_framerate, delta_t);
    };

    Ok(())
}
