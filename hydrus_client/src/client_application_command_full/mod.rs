//! Complete simple-command id table generated from ClientApplicationCommand.py.

mod ids_1;
mod ids_2;
mod ids_3;
mod names_1;
mod names_2;

pub use ids_1::*;
pub use ids_2::*;
pub use ids_3::*;

pub fn simple_command_name(command_type: i32) -> &'static str {
    if command_type <= 79 {
        names_1::simple_command_name(command_type)
    } else {
        names_2::simple_command_name(command_type)
    }
}
