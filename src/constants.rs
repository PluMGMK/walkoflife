/*!
  Important pointers in Rayman 2's memory, as found in Robin's
  [Constants.cs](https://github.com/rtsonneveld/Rayman2FunBox/blob/master/Rayman2FunBox/Constants.cs).
  */

pub const OFF_DNM_P_ST_DYNAMICS_CAMERA_MECHANICS: usize = 0x4359D0;
pub const OFF_FORCE_CAMERA_POS: usize = 0x473420;
pub const OFF_FORCE_CAMERA_TGT: usize = 0x473480;

pub const OFF_ENGINE_STRUCTURE: usize = 0x500380;
pub const OFF_ENGINE_MODE: usize = OFF_ENGINE_STRUCTURE;
pub const OFF_LEVEL_NAME: usize = OFF_ENGINE_STRUCTURE + 0x1F;
pub const OFF_HEALTH_PTR_1: usize = 0x500584;
pub const OFF_VOID_PTR: usize = 0x4B9BC8;
pub const OFF_BRIGHTNESS_PTR: usize = 0x4A0488;
pub const OFF_CAMERA_ARRAY_PTR: usize = 0x500550;
pub const OFF_MAIN_CHAR: usize = 0x500578;
pub const OFF_TURN_FACTOR: usize = 0x49CC3C;

pub const OFF_INPUT_X: usize = 0x4B9BA0;
pub const OFF_INPUT_Y: usize = 0x4B9BA4;

pub const OFF_OBJECT_TYPES: usize = 0x005013E0;
