use core::ffi::c_void;

use spin::Once;

use crate::Spa;

/// A set of platform API needed during installation of the hypervisor.
///
/// Calling any of those after set up from the host context is unsound. It is a
/// "call-out" from the host to guest and violates the security boundary.
struct OsApi {
    to_spa: fn(va: *const c_void) -> Spa,
    to_va: fn(spa: Spa) -> *mut c_void,
}

static OS_API: Once<OsApi> = Once::new();

pub(crate) fn init(to_spa: fn(va: *const c_void) -> Spa, to_va: fn(spa: Spa) -> *mut c_void) {
    let _ = OS_API.call_once(|| OsApi { to_spa, to_va });
}

impl<T> From<&mut T> for Spa {
    fn from(reference: &mut T) -> Self {
        Spa::from(core::ptr::from_ref(reference as &T))
    }
}

impl<T> From<*const T> for Spa {
    fn from(ptr: *const T) -> Self {
        let to_spa = OS_API.get().expect("OS_API is initialized").to_spa;
        to_spa(ptr.cast())
    }
}

impl Spa {
    #[expect(dead_code)]
    pub(crate) fn to_va(self) -> *mut c_void {
        let to_va = OS_API.get().expect("OS_API is initialized").to_va;
        to_va(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[ctor::ctor]
    fn init() {
        fn to_spa<T>(ptr: *const T) -> Spa {
            Spa::new(ptr as _)
        }

        fn to_va(spa: Spa) -> *mut c_void {
            spa.as_u64() as _
        }

        super::init(to_spa, to_va);
    }
}
