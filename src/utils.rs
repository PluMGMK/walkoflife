/*!
  Various useful functions for dealing with Rayman 2's engine. Most of these come from Robin's
  [Utils.cs](https://github.com/rtsonneveld/Rayman2FunBox/blob/master/Rayman2FunBox/Utils.cs).
  */

extern crate nix;

use std::{process::Command,collections::HashMap};
use nix::{libc::pid_t,unistd::Pid};
use crate::{memory::{read_prims,read_string,get_pointer_path},constants::*};

fn find_rayman2_pidof() -> Result<Pid,&'static str> {
    if let Ok(out) = Command::new("pidof").arg("Rayman2.exe").output() {
        if let Ok(strout) = String::from_utf8(out.stdout) {
            if let Some(firstline) = strout.lines().next() {
                if let Ok(num) = firstline.parse::<pid_t>() {
                    Ok(Pid::from_raw(num))
                } else {
                    Err("No numerical output from pidof")
                }
            } else {
                Err("Got no output from pidof")
            }
        } else {
            Err("Failed to parse output of pidof")
        }
    } else {
        Err("Failed to run pidof")
    }
}

fn find_rayman2_pgrep() -> Result<Pid,&'static str> {
    if let Ok(out) = Command::new("pgrep").arg("Rayman2.exe").output() {
        if let Ok(strout) = String::from_utf8(out.stdout) {
            if let Some(firstline) = strout.lines().next() {
                if let Ok(num) = firstline.parse::<pid_t>() {
                    Ok(Pid::from_raw(num))
                } else {
                    Err("No numerical output from pgrep")
                }
            } else {
                Err("Got no output from pgrep")
            }
        } else {
            Err("Failed to parse output of pgrep")
        }
    } else {
        Err("Failed to run pgrep")
    }
}

/// Find the PID of the currently-running `Rayman2.exe` process.
///
/// ## Requirements:
/// * Rayman 2 needs to be running, and the filename used to launch it needs to be `Rayman2.exe`.
/// * Either `pidof` or `pgrep` needs to be in the `PATH` of this program's environment.
/// (Preferably the latter.)
///
/// ## Returns:
/// * On success (i.e. if the PID was found), returns a
/// [nix::unistd::Pid](../../nix/unistd/struct.Pid.html) corresponding to the running Rayman 2
/// process.
/// * Returns an `Err` variant with a text description of what went wrong on failure.
pub fn find_attach_rayman2() -> Result<Pid,String> {
    match find_rayman2_pidof() {
        Ok(pid) => Ok(pid),
        Err(err) => {
            println!("Couldn't find Rayman 2 with pidof - {}", err);
            print!("Trying pgrep instead... ");
            
            match find_rayman2_pgrep() {
                Ok(pid) => {
                    println!("OK!");
                    Ok(pid)
                },
                Err(err) => Err(err.into()),
            }
        },
    }
}

/// Get the environment of the process given by `r2pid`, as a `HashMap`.
///
/// ## Requirements:
/// * This program needs to have permission to read `/proc/<r2pid>/environ`.
///
/// ## Returns:
/// * On success, returns a
/// [`HashMap`](https://doc.rust-lang.org/std/collections/struct.HashMap.html)
/// with keys corresponding to environment variables and values equal to their values.
/// * Returns an `Err` variant with a text description of what went wrong on failure.
pub fn get_environment(r2pid:Pid) -> Result<HashMap<String,String>, String> {
    let env_buf = match std::fs::read(format!("/proc/{}/environ", r2pid)) {
        Ok(buf) => buf,
        Err(err) => {
            return Err(format!("Unable to open Rayman 2's environment file: {:?}", err));
        },
    };

    let mut ret = HashMap::new();
    let mut buf_iter = env_buf.into_iter().peekable();
    while buf_iter.peek() != None {
        let key = match String::from_utf8(
            buf_iter.by_ref()
            .take_while(|&x| x != b'=') // Everything before the first equals sign is the key.
            .collect() ) {
            Ok(string) => string,
            Err(_) => {continue;}, // Don't bring down the whole thing...
        };
        let val = match String::from_utf8(
            buf_iter.by_ref()
            .take_while(|&x| x != 0) // The rest of the string, up to the null-terminator, is the value.
            .collect() ) {
            Ok(string) => string,
            Err(_) => {continue;}, // Don't bring down the whole thing...
        };

        ret.insert(key, val);
    }

    Ok(ret)
}

/// Send some fake X11 input to the display given by `disp`, using the `xte` program from
/// [`xautomation`](https://www.hoopajoo.net/projects/xautomation.html). This is used to implement
/// auto-strafing when the down button is pressed in FPS mode.
///
/// ## Requirements:
/// * Rayman 2 should be running on the X display given in `disp`.
/// * `xte` needs to be in the `PATH` of this program's environment.
/// * `command` should be a valid option for `xte` - see 
/// [its man page](https://linux.die.net/man/1/xte) for details.
///
/// ## Returns:
/// * On success, returns `Ok(())`.
/// * Returns an `Err` variant with a text description of what went wrong on failure.
pub fn send_input(disp: &str, command: &str) -> Result<(), String> {
    if let Err(err) = Command::new("xte")
        .args(&["-x", &disp, command])
            .spawn() {
                Err(format!("Couldn't send input to Rayman 2 with xte: {:?}", err))
            }
    else {
        Ok(())
    }
}

/// Read the name of the level currently open in the Rayman 2 process given by `r2pid`.
///
/// ## Requirements:
/// * We need to have permissions to debug `pid` (e.g. with `CAP_SYS_PTRACE`).
///
/// ## Returns:
/// * On success, returns the level name as a `String`.
/// * Returns an `Err` variant with a text description of what went wrong,
/// if the memory read fails.
pub fn get_current_level_name(r2pid:Pid) -> Result<String,String> {
    match read_string(r2pid, OFF_LEVEL_NAME, 16) {
        Ok(name) => Ok(name),
        Err(err) => Err(format!("Couldn't read level name: {:?}", err)),
    }
}

/// Get the index in the hierarchy of a family at memory position `offset_family`, in
/// process given by `r2pid`.
///
/// ## Requirements:
/// * We need to have permissions to debug `pid` (e.g. with `CAP_SYS_PTRACE`).
/// * You need to give a pointer to a valid family.
///
/// ## Returns:
/// * On success, returns the index of the given family.
/// * Returns an `Err` variant with a text description of what went wrong,
/// if the memory read fails.
pub fn get_family_index(r2pid: Pid, off_family: usize) -> Result<usize, String> {
    match get_pointer_path(r2pid, off_family + 0xC, None) {
        Ok(ptr) => Ok(ptr),
        Err(err) => Err(format!("Couldn't get family index: {:?}", err))
    }
}

/// Get the PO meshes (and pointers thereto) for a family at memory position `offset_family`, in
/// process given by `r2pid`, optionally discarding those with certain `indices`.
///
/// ## Requirements:
/// * We need to have permissions to debug `pid` (e.g. with `CAP_SYS_PTRACE`).
/// * You need to give a pointer to a valid family.
///
/// ## Returns:
/// * On success, returns a
/// [`HashMap`](https://doc.rust-lang.org/std/collections/struct.HashMap.html).
///     * The keys are pointers to the PO meshes in the given family.
///     * The values are `Vec<f32>`s containing all the vertices of the meshes as of when they were
///     read from memory. Of course, each group of three floats in the vector is a single vertex.
///     * Note that you can skip certain POs in the family by specifying their `indices`.
///     Alternatively, you can choose to keep only certain POs by specifying `keep_instead = true`.
/// * Returns an `Err` variant with a text description of what went wrong,
/// if the memory read fails.
pub fn get_family_po_vert_offsets(r2pid:Pid, offset_family:usize, keep_instead:bool, indices:&Vec<usize>) -> Result<HashMap<usize,Vec<f32>>, String> {
    let mut ret = HashMap::new();

    let off_default_objects_table = match get_pointer_path(r2pid, offset_family + 0x1C, None) {
        Ok(ptr) => ptr,
        Err(err) => {return Err(format!("Couldn't get default object table offset: {:?}", err));},
    };

    let (first_entry, num_entries) = match read_prims::<u32>(r2pid, off_default_objects_table+4, 3) {
        Ok(vec) => (vec[0] as usize, vec[2] as usize), // Want the pointers at off_default_objects_table + 0x4 and + 0xC
        Err(err) => {return Err(format!("Couldn't find address or number of entries in object table: {:?}", err));},
    };

    for i in 0..num_entries {
        let cur_entry = first_entry + (i * 0x14);

        if indices.contains(&i) == keep_instead {
            continue;
        }

        let off_visualset = match get_pointer_path(r2pid, cur_entry + 4, Some(&vec![0])) {
            Ok(ptr) => ptr,
            Err(_) => {continue;}, // Apparently this CAN fail with impunity...
        };

        let (num_of_lod, visual_type) = match read_prims::<i16>(r2pid, off_visualset + 4, 2) {
            Ok(vec) => (vec[0], vec[1]),
            Err(_) => {continue;}, // Apparently this CAN fail with impunity...
        };

        if num_of_lod > 0 && visual_type == 0 {
            let off_first_mesh = match get_pointer_path(r2pid, off_visualset + 0xC, Some(&vec![0])) {
                Ok(ptr) => ptr,
                Err(_) => {continue;},
            };
            let off_first_mesh_num_vertices = off_first_mesh + 0x2C;
            //let off_first_mesh_num_sub_blocks = off_first_mesh + 0x2E;
            //let off_first_mesh_sub_blocks = off_first_mesh + 0x14;
            //let off_first_mesh_sub_block_types = off_first_mesh + 0x10;
            let off_verts = match get_pointer_path(r2pid, off_first_mesh, None) {
                Ok(ptr) => ptr,
                Err(_) => {continue;},
            };

            /*let num_sub_blocks = match read_prims::<i16>(r2pid, off_first_mesh_num_sub_blocks, 1) {
                Ok(vec) => vec[0],
                Err(err) => {return Err(format!("Couldn't get number of subblocks: {:?}", err));}
            };*/
            let num_verts = match read_prims::<i16>(r2pid, off_first_mesh_num_vertices, 1) {
                Ok(vec) => vec[0],
                Err(err) => {return Err(format!("Couldn't get number of vertices: {:?}", err));}
            };

            // Each vertex is naturally three floats
            let all_verts = match read_prims::<f32>(r2pid, off_verts, 3 * num_verts as usize) {
                Ok(vec) => vec,
                Err(err) => {return Err(format!("Couldn't get vertex positions: {:?}", err));}
            };
            ret.insert(off_verts, all_verts); // Put vectors in the HashMap - it'll be more efficient...
        }
    }

    Ok(ret)
}

/// Look up the names of a certain number of objects in the engine hierarchy of the Rayman 2
/// process given by `r2pid`, starting from a known object.
///
/// ## Requirements:
/// * We need to have permissions to debug `pid` (e.g. with `CAP_SYS_PTRACE`).
/// * `off_names_first` needs to correspond to a valid object.
/// * You need to know how many objects you want to go through, specified by `num_names`.
///
/// ## Returns:
/// * A `Vec<String>` with `len()` equal to `num_names`. This is guaranteed, but it may contain
/// blanks or repeats if the function input was not sane.
pub fn read_object_names_table(r2pid: Pid, off_names_first: usize, num_names: usize) -> Vec<String> {
    let mut cur_offset = off_names_first;
    let mut ret = Vec::with_capacity(num_names);

    for _j in 0..num_names {
        let res_off_names_next = get_pointer_path(r2pid, cur_offset, None);

        if let Ok(off_name) = get_pointer_path(r2pid, cur_offset + 0xC, None) {
            ret.push(
                match read_string(r2pid, off_name, 64) {
                    Ok(name) => name,
                    Err(_) => "".into(),
                });
        } else {
            // I'm guessing this can also fail with impunity...
            ret.push("".into());
        }

        if let Ok(off_names_next) = res_off_names_next {
            if off_names_next > 0 {
                cur_offset = off_names_next;
            }
        }
    }

    ret
}

/// Read all the object types in the engine hierarchy of Rayman 2 process given by `r2pid`.
///
/// ## Requirements:
/// * We need to have permissions to debug `pid` (e.g. with `CAP_SYS_PTRACE`).
///
/// ## Returns:
/// * On success, returns an array of three `Vec<String>`s. The first one contains the family
/// names, the second one contains the AI Model names, and the third contains the super-object
/// names.
/// * Returns an `Err` variant with a text description of what went wrong,
/// if the memory read fails.
pub fn read_object_types(r2pid: Pid) -> Result<[Vec<String>; 3], String> {
    let mut iter = ["family", "AI Model", "super-object"]
        .iter()
        .enumerate()
        .map(|(i, desc)| {
            let off_names_header = OFF_OBJECT_TYPES + i*12;
            let (off_names_first, _off_names_last, num_names) = 
                match read_prims::<u32>(r2pid, off_names_header, 3) {
                    Ok(vec) => (vec[0] as usize, vec[1] as usize, vec[2] as usize),
                    Err(err) => {return Err(format!("Unable to read {} names: {:?}", desc, err));},
                };

            Ok(read_object_names_table(r2pid, off_names_first, num_names))
        });

    // iter is guaranteed to give three elements. We call unwrap() on the result of next() three
    // times to get all three of them. The question marks bubble up the "Unable to read names"
    // errors.
    Ok([
       iter.next().unwrap()?,
       iter.next().unwrap()?,
       iter.next().unwrap()?
    ])
}

/// Get the names and memory locations of all active super-objects in the engine hierarchy of the
/// Rayman 2 process given by `r2pid`, starting from a given `super_object` pointer (or the dynamic
/// world itself if that is set to 0).
///
/// ## Requirements:
/// * We need to have permissions to debug `pid` (e.g. with `CAP_SYS_PTRACE`).
/// * You need to know the list of super-object names in the hierarchy and pass it via the argument
/// `object_names`. This list can be obtained with
/// [`read_object_types()`](fn.read_object_types.html)`.unwrap()[2]`.
///
/// ## Returns:
/// * On success, returns a
/// [`HashMap`](https://doc.rust-lang.org/std/collections/struct.HashMap.html).
///     * The keys are the names of the super-objects.
///     * The values are pointers to the super-objects in Rayman 2's memory.
/// * Returns an `Err` variant with a text description of what went wrong,
/// if the memory read fails.
pub fn get_active_super_object_names(r2pid: Pid, object_names: &Vec<String>, super_object: usize) -> Result<HashMap<String,usize>, String> {
    let mut ret = HashMap::new();
    let super_object = match super_object {
        0 => {
            let off_dynam_world = 0x500FD0;
            match get_pointer_path(r2pid, off_dynam_world, Some(&vec![8])) {
                Ok(ptr) => ptr,
                Err(err) => {return Err(format!("Couldn't get super-object for dynamic world: {:?}", err));},
            }
        },
        val => val,
    };

    let mut next_brother = super_object;

    loop {
        if next_brother != 0 {
            let name_index = match get_pointer_path(r2pid, next_brother + 4, Some(&vec![4, 8])) {
                Ok(ptr) => ptr,
                Err(_) => {break;},
            };
            let name = match object_names.get(name_index) {
                Some(namestr) => namestr.to_string(),
                None => format!("unknown_{}", next_brother),
            };
            ret.insert(name, next_brother);
        } else {
            break;
        }

        next_brother = match get_pointer_path(r2pid, next_brother + 0x14, None) {
            Ok(ptr) => ptr,
            Err(_) => {break;},
        };
    }

    Ok(ret)
}

/// Get the names of AI Models and lists of memory locations of all corresponding active super-objects
/// in the engine hierarchy of the Rayman 2 process given by `r2pid`, starting from a given
/// `super_object` pointer (or the dynamic world itself if that is set to 0).
///
/// ## Requirements:
/// * We need to have permissions to debug `pid` (e.g. with `CAP_SYS_PTRACE`).
/// * You need to know the list of AI Model names in the hierarchy and pass it via the argument
/// `object_names`. This list can be obtained with
/// [`read_object_types()`](fn.read_object_types.html)`.unwrap()[1]`.
///
/// ## Returns:
/// * On success, returns a
/// [`HashMap`](https://doc.rust-lang.org/std/collections/struct.HashMap.html).
///     * The keys are the names of the AI Models.
///     * The values are vectors of pointers to the corresponding super-objects in Rayman 2's memory.
/// * Returns an `Err` variant with a text description of what went wrong,
/// if the memory read fails.
pub fn get_active_super_object_ai_model_names(r2pid: Pid, ai_model_names: &Vec<String>, super_object: usize) -> Result<HashMap<String,Vec<usize>>,String> {
    let mut ret: HashMap<String,Vec<usize>> = HashMap::new();
    let super_object = match super_object {
        0 => {
            let off_dynam_world = 0x500FD0;
            match get_pointer_path(r2pid, off_dynam_world, Some(&vec![8])) {
                Ok(ptr) => ptr,
                Err(err) => {return Err(format!("Couldn't get super-object for dynamic world: {:?}", err));},
            }
        },
        val => val,
    };

    let mut next_brother = super_object;

    loop {
        if next_brother != 0 {
            let name_index = match get_pointer_path(r2pid, next_brother + 4, Some(&vec![4, 4])) {
                Ok(ptr) => ptr,
                Err(_) => {break;},
            };
            let name = match ai_model_names.get(name_index) {
                Some(namestr) => namestr.to_string(),
                None => format!("unknown_{}", next_brother),
            };
            if ret.contains_key(&name) {
                ret.get_mut(&name).unwrap().push(next_brother);
            } else {
                ret.insert(name, vec![next_brother]);
            }
        } else {
            break;
        }

        next_brother = match get_pointer_path(r2pid, next_brother + 0x14, None) {
            Ok(ptr) => ptr,
            Err(_) => {break;},
        };
    }

    Ok(ret)
}

/// Get a pointer to the mind object of the given `super_object`
/// in the Rayman 2 process given by `r2pid`.
///
/// ## Requirements:
/// * We need to have permissions to debug `pid` (e.g. with `CAP_SYS_PTRACE`).
/// * You need to give a pointer to a valid super-object.
///
/// ## Returns:
/// * On success, returns a pointer to the mind object for the given super-object.
/// * Returns an `Err` variant with a text description of what went wrong,
/// if the memory read fails.
pub fn get_mind(r2pid: Pid, super_object: usize) -> Result<usize, String> {
    match get_pointer_path(r2pid, super_object + 4, Some(&vec![0xC, 0])) {
        Ok(ptr) => Ok(ptr),
        Err(err) => Err(format!("Unable to get Mind: {:?}", err)),
    }
}

/// Get the index of the currently-active behaviour (comport) on the given `super_object`
/// in the Rayman 2 process given by `r2pid`.
///
/// ## Requirements:
/// * We need to have permissions to debug `pid` (e.g. with `CAP_SYS_PTRACE`).
/// * You need to give a pointer to a valid super-object.
///
/// ## Returns:
/// * On success, returns the index of the active comport.
/// * Returns an `Err` variant with a text description of what went wrong,
/// if the memory read fails.
pub fn get_active_normal_behaviour(r2pid: Pid, super_object: usize) -> Result<usize, String> {
    let off_mind = get_mind(r2pid, super_object)?;
    match get_pointer_path(r2pid, off_mind + 4, Some(&vec![0x8])) {
        Ok(ptr) => Ok(ptr),
        Err(err) => Err(format!("Unable to get Active Normal Behaviour: {:?}", err)),
    }
}

/// Get a pointer to a certain DSG variable on the given `super_object`
/// in the Rayman 2 process given by `r2pid`.
///
/// ## Requirements:
/// * We need to have permissions to debug `pid` (e.g. with `CAP_SYS_PTRACE`).
/// * You need to give a pointer to a valid super-object.
/// * You need to know the `offset` of the DSG variable you want (can't currently address DSG
/// variables by index). You can find this in Raymap by clicking "Print DsgVar from Mind->DsgMem"
/// under the "Perso Behaviour" component of the object you're interested in.
///
/// ## Returns:
/// * On success, returns a pointer to the desired DSG variable.
/// * Returns an `Err` variant with a text description of what went wrong,
/// if the memory read fails.
pub fn get_dsg_var_ptr(r2pid: Pid, super_object: usize, offset: usize) -> Result<usize, String> {
    let off_mind = get_mind(r2pid, super_object)?;
    match get_pointer_path(r2pid, off_mind + 0xC, Some(&vec![8])) {
        Ok(ptr) => Ok(ptr + offset),
        Err(err) => Err(format!("Unable to get DSG Var pointer: {:?}", err)),
    }
}

/// Get a pointer to the custom bits of the given `super_object`
/// in the Rayman 2 process given by `r2pid`.
///
/// ## Requirements:
/// * We need to have permissions to debug `pid` (e.g. with `CAP_SYS_PTRACE`).
/// * You need to give a pointer to a valid super-object.
///
/// ## Returns:
/// * On success, returns a pointer to the custom bits.
/// * Returns an `Err` variant with a text description of what went wrong,
/// if the memory read fails.
pub fn get_custom_bits_ptr(r2pid: Pid, super_object: usize) -> Result<usize, String> {
    match get_pointer_path(r2pid, super_object + 4, Some(&vec![4])) {
        Ok(ptr) => Ok(ptr + 0x24),
        Err(err) => Err(format!("Unable to get Custom Bits: {:?}", err)),
    }
}

/// Get a pointer to the AI Model used by the given `super_object`
/// in the Rayman 2 process given by `r2pid`.
///
/// ## Requirements:
/// * We need to have permissions to debug `pid` (e.g. with `CAP_SYS_PTRACE`).
/// * You need to give a pointer to a valid super-object.
///
/// ## Returns:
/// * On success, returns a pointer to the custom bits.
/// * Returns an `Err` variant with a text description of what went wrong,
/// if the memory read fails.
pub fn get_custom_bits_ptr(r2pid: Pid, super_object: usize) -> Result<usize, String> {
    match get_pointer_path(r2pid, super_object + 4, Some(&vec![4])) {
        Ok(ptr) => Ok(ptr + 0x24),
        Err(err) => Err(format!("Unable to get Custom Bits: {:?}", err)),
    }
}

/// Get a pointer to the AI Model used by the given `super_object`
/// in the Rayman 2 process given by `r2pid`.
///
/// ## Requirements:
/// * We need to have permissions to debug `pid` (e.g. with `CAP_SYS_PTRACE`).
/// * You need to give a pointer to a valid super-object.
///
/// ## Returns:
/// * On success, returns a pointer to the AI Model.
/// * Returns an `Err` variant with a text description of what went wrong,
/// if the memory read fails.
pub fn get_ai_model(r2pid: Pid, super_object: usize) -> Result<usize, String> {
    let off_mind = get_mind(r2pid, super_object)?;
    //match get_pointer_path(r2pid, super_object + 4, Some(&vec![0xC, 0, 0])) {
    match get_pointer_path(r2pid, off_mind, None) {
        Ok(ptr) => Ok(ptr),
        Err(err) => Err(format!("Unable to get AI Model pointer: {:?}", err)),
    }
}

/// Get a pointer to the vector of normal behaviours (comports) in the AI Model used by the given `super_object`
/// in the Rayman 2 process given by `r2pid`.
///
/// ## Requirements:
/// * We need to have permissions to debug `pid` (e.g. with `CAP_SYS_PTRACE`).
/// * You need to give a pointer to a valid super-object.
///
/// ## Returns:
/// * On success, returns a pointer to the vector of normal behaviours.
/// * Returns an `Err` variant with a text description of what went wrong,
/// if the memory read fails.
pub fn get_ai_model_normal_behaviours_ptr(r2pid: Pid, super_object: usize) -> Result<usize, String> {
    let ai_model = get_ai_model(r2pid, super_object)?;
    match get_pointer_path(r2pid, ai_model, None) {
        Ok(ptr) => Ok(ptr),
        Err(err) => Err(format!("Unable to get AI Model Normal Behaviours pointer: {:?}", err)),
    }
}

/// Get a list of pointers to the normal behaviours (comports) in the AI Model used by the given `super_object`
/// in the Rayman 2 process given by `r2pid`.
///
/// ## Requirements:
/// * We need to have permissions to debug `pid` (e.g. with `CAP_SYS_PTRACE`).
/// * You need to give a pointer to a valid super-object.
///
/// ## Returns:
/// * On success, returns a `Vec<usize>` of pointers to the normal behaviours.
/// * Returns an `Err` variant with a text description of what went wrong,
/// if the memory read fails.
pub fn get_ai_model_normal_behaviours_list(r2pid: Pid, super_object: usize) -> Result<Vec<usize>, String> {
    let offset = get_ai_model_normal_behaviours_ptr(r2pid, super_object)?;
    let (off_first_entry, num_entries) = match read_prims::<u32>(r2pid, offset, 2) {
        Ok(vec) => (vec[0] as usize, vec[1] as usize),
        Err(err) => {return Err(format!("Unable to get entries in AI Model Normal Behaviours List: {:?}", err));},
    };

    // Each entry takes up 12 bytes.
    Ok((0..num_entries).map(|i| off_first_entry + 12*i).collect())
}
