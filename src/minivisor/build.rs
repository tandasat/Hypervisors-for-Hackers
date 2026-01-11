//! Specifies the way to build the Windows driver using the wdk-build crate.

fn main() -> Result<(), wdk_build::ConfigError> {
    wdk_build::configure_wdk_binary_build()
}
