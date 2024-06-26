// generate ffi bindings

mod local_file_system;
#[cfg(feature = "memmap")]
mod memmap;
#[cfg(feature = "sqlite")]
mod sqlite;
