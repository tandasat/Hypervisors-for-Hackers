use core::{arch::asm, ptr::addr_of_mut};

use derive_more::{Display, Error};

#[derive(Debug, Display, Error)]
pub(crate) enum Error {
    #[display("`{selector:#x?}` points to the null descriptor")]
    NullDescriptor { selector: Selector },

    #[display("`{selector:#x?}` points to LDT where parsing is unimplemented")]
    LdtAccess { selector: Selector },

    #[display("`{index}` points to outside GDT")]
    OutOfGdtAccess { index: usize },

    #[display("`{index}` points to `{entry}`, which is invalid as a descriptor")]
    InvalidGdtEntry { index: usize, entry: u64 },
}

pub(crate) struct Descriptor {
    low64: DescriptorLow64,
    upper_base: Option<u32>,
}

impl Descriptor {
    pub(crate) fn try_new(gdtr: &Gdtr, selector: Selector) -> Result<Self, Error> {
        if selector.ti() {
            return Err(Error::LdtAccess { selector });
        }

        let index = selector.index() as usize;
        if index == 0 {
            return Err(Error::NullDescriptor { selector });
        }

        let gdt = unsafe {
            core::slice::from_raw_parts(gdtr.base as *const u64, usize::from(gdtr.limit + 1) / 8)
        };

        let raw = gdt.get(index).ok_or(Error::OutOfGdtAccess { index })?;

        let low64 = DescriptorLow64(*raw);
        let upper_base = if low64.is_16byte() {
            let index = index + 1;

            let raw = gdt.get(index).ok_or(Error::OutOfGdtAccess { index })?;

            let Ok(upper_base) = u32::try_from(*raw) else {
                return Err(Error::InvalidGdtEntry { index, entry: *raw });
            };

            Some(upper_base)
        } else {
            None
        };
        Ok(Self { low64, upper_base })
    }

    pub(crate) fn base(&self) -> u64 {
        if let Some(upper_base) = self.upper_base {
            u64::from(self.low64.base()) | (u64::from(upper_base) << 32)
        } else {
            self.low64.base().into()
        }
    }
}

bitfield::bitfield! {
    /// Figure 3-8. Segment Descriptor
    #[derive(Clone, Copy, Debug)]
    struct DescriptorLow64(u64);

    /// Segment Limit 15:00 - Specifies the size of the segment.
    segment_limit_low, set_segment_limit_low: 15, 0;

    /// Base Address 23:00 - Defines the location of byte 0 of the segment within the 4-GByte linear
    /// address space.
    base_low, set_base_low: 39, 16;

    /// Indicates the segment or gate type and specifies the kinds of access that
    /// can be made to the segment and the direction of growth.
    type_, set_type: 43, 40;

    /// Specifies whether the segment descriptor is for a system segment (S flag
    /// is clear) or a code or data segment (S flag is set).
    descriptor_type, set_descriptor_type: 44;

    /// DPL (descriptor privilege level) field - Specifies the privilege level of
    /// the segment.
    dpl, set_dpl: 46, 45;

    /// P (segment-present) flag - Indicates whether the segment is present in
    /// memory (set) or not present (clear).
    present, set_present: 47;

    /// Segment Limit 19:16 - Specifies the size of the segment.
    segment_limit_high, set_segment_limit_high: 51, 48;

    /// Available for use by system software.
    available, set_available: 52;

    /// L (64-bit code segment) flag - In IA-32e mode, bit 21 of the second
    /// doubleword of the segment descriptor indicates whether a code segment
    /// contains native 64-bit code.
    large, set_large: 53;

    /// D/B (default operation size/default stack pointer size and/or upper bound)
    /// flag - Performs different functions depending on whether the segment
    /// descriptor is an executable code segment, an expand-down data segment,
    /// or a stack segment.
    default_big, set_default_big: 54;

    /// G (granularity) flag - Determines the scaling of the segment limit field.
    granularity, set_granularity: 55;

    /// Base Address 31:24 - Defines the location of byte 0 of the segment within
    /// the 4-GByte linear address space.
    base_high, set_base_high: 63, 56;
}

impl DescriptorLow64 {
    // "In 64-bit mode, the TSS descriptor is expanded to 16 bytes (...)."
    // See: 9.2.3 TSS Descriptor in 64-bit mode
    fn is_16byte(self) -> bool {
        const TSS_TYPE_AVAILABLE: u64 = 0x9;
        const TSS_TYPE_BUSY: u64 = 0xb;

        let type_ = self.type_();
        !self.descriptor_type() && (type_ == TSS_TYPE_AVAILABLE || type_ == TSS_TYPE_BUSY)
    }

    fn base(self) -> u32 {
        let base_low = self.base_low();
        let base_high = self.base_high();
        u32::try_from((base_high << 24) | base_low).unwrap()
    }
}

/// Returns access rights in the format VMCS expects.
pub(crate) fn vmx_access_rights(access_rights: u32) -> u32 {
    const VMX_SEGMENT_ACCESS_RIGHTS_UNUSABLE_FLAG: u32 = 1 << 16;

    // "In general, a segment register is unusable if it has been loaded with a
    //  null selector."
    // See: 26.4.1 Guest Register State
    if access_rights == 0 {
        return VMX_SEGMENT_ACCESS_RIGHTS_UNUSABLE_FLAG;
    }

    // Convert the native access right to the format for VMX. Those two formats
    // are almost identical except that first 8 bits of the native format does
    // not exist in the VMX format, and that few fields are undefined in the
    // native format but reserved to be zero in the VMX format.
    (access_rights >> 8) & 0b1111_0000_1111_1111
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(C, packed)]
pub(crate) struct Gdtr {
    pub(crate) limit: u16,
    pub(crate) base: u64,
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(C, packed)]
pub(crate) struct Idtr {
    pub(crate) limit: u16,
    pub(crate) base: u64,
}

pub(crate) unsafe fn sgdt() -> Gdtr {
    unsafe {
        let mut gdtr = Gdtr::default();
        asm!("sgdt [{}]", in(reg) addr_of_mut!(gdtr), options(nostack, preserves_flags));
        gdtr
    }
}

pub(crate) unsafe fn sidt() -> Idtr {
    unsafe {
        let mut idtr = Idtr::default();
        asm!("sidt [{}]", in(reg) addr_of_mut!(idtr), options(nostack, preserves_flags));
        idtr
    }
}

pub(crate) fn cs() -> Selector {
    let segment;
    unsafe { asm!("mov {:x}, cs", out(reg) segment, options(nomem, nostack, preserves_flags)) };
    Selector(segment)
}

pub(crate) fn es() -> Selector {
    let segment;
    unsafe { asm!("mov {:x}, es", out(reg) segment, options(nomem, nostack, preserves_flags)) };
    Selector(segment)
}

pub(crate) fn ss() -> Selector {
    let segment;
    unsafe { asm!("mov {:x}, ss", out(reg) segment, options(nomem, nostack, preserves_flags)) };
    Selector(segment)
}

pub(crate) fn ds() -> Selector {
    let segment;
    unsafe { asm!("mov {:x}, ds", out(reg) segment, options(nomem, nostack, preserves_flags)) };
    Selector(segment)
}

pub(crate) fn fs() -> Selector {
    let segment;
    unsafe { asm!("mov {:x}, fs", out(reg) segment, options(nomem, nostack, preserves_flags)) };
    Selector(segment)
}

pub(crate) fn gs() -> Selector {
    let segment;
    unsafe { asm!("mov {:x}, gs", out(reg) segment, options(nomem, nostack, preserves_flags)) };
    Selector(segment)
}

pub(crate) fn tr() -> Selector {
    let segment;
    unsafe { asm!("str {:x}", out(reg) segment, options(nomem, nostack, preserves_flags)) };
    Selector(segment)
}

pub(crate) fn ldtr() -> Selector {
    let segment;
    unsafe { asm!("sldt {:x}", out(reg) segment, options(nomem, nostack, preserves_flags)) };
    Selector(segment)
}

/// LSL-Load Segment Limit
pub(crate) fn lsl(selector: Selector) -> u32 {
    const RFLAGS_ZF: u64 = 1 << 6;

    let flags: u64;
    let mut limit: u64;
    unsafe {
        asm!(
            "lsl {}, {}",
            "pushfq",
            "pop {}",
            out(reg) limit,
            in(reg) u64::from(selector.0),
            lateout(reg) flags
        );
    };
    if flags & RFLAGS_ZF != 0 {
        limit as _
    } else {
        0
    }
}

/// LAR-Load Access Rights Byte
pub(crate) fn lar(selector: Selector) -> u32 {
    const RFLAGS_ZF: u64 = 1 << 6;

    let flags: u64;
    let mut access_rights: u64;
    unsafe {
        asm!(
            "lar {}, {}",
            "pushfq",
            "pop {}",
            out(reg) access_rights,
            in(reg) u64::from(selector.0),
            lateout(reg) flags
        );
    };
    if flags & RFLAGS_ZF != 0 {
        access_rights as _
    } else {
        0
    }
}

bitfield::bitfield! {
    /// 3.4.2 Segment Selectors
    #[derive(Clone, Copy, Debug)]
    pub(crate) struct Selector(u16);

    /// Requested Privilege Level (RPL) - Specifies the privilege level of the
    /// selector.
    rpl, set_rpl: 1, 0;

    /// TI (table indicator) flag - Specifies the descriptor table to use: clearing
    /// this flag selects the GDT; setting this flag selects the current LDT.
    ti, set_ti: 2;

    /// Index - Selects one of 8192 descriptors in the GDT or LDT.
    index, set_index: 15, 3;
}

#[cfg(test)]
mod tests {
    use core::ptr::addr_of;

    use super::*;

    #[test]
    fn descriptor() {
        /*
            kd> dg 0 60
                                                                P Si Gr Pr Lo
            Sel        Base              Limit          Type    l ze an es ng Flags
            ---- ----------------- ----------------- ---------- - -- -- -- -- --------
            0000 00000000`00000000 00000000`00000000 <Reserved> 0 Nb By Np Nl 00000000
            0008 00000000`00000000 00000000`00000000 <Reserved> 0 Nb By Np Nl 00000000
            0010 00000000`00000000 00000000`00000000 Code RE Ac 0 Nb By P  Lo 0000029b
            0018 00000000`00000000 00000000`00000000 Data RW Ac 0 Bg By P  Nl 00000493
            0020 00000000`00000000 00000000`ffffffff Code RE Ac 3 Bg Pg P  Nl 00000cfb
            0028 00000000`00000000 00000000`ffffffff Data RW Ac 3 Bg Pg P  Nl 00000cf3
            0030 00000000`00000000 00000000`00000000 Code RE Ac 3 Nb By P  Lo 000002fb
            0038 00000000`00000000 00000000`00000000 <Reserved> 0 Nb By Np Nl 00000000
            0040 00000000`71e7b000 00000000`00000067 TSS32 Busy 0 Nb By P  Nl 0000008b
            0048 00000000`0000ffff 00000000`0000f805 <Reserved> 0 Nb By Np Nl 00000000
            0050 00000000`00000000 00000000`00003c00 Data RW Ac 3 Bg By P  Nl 000004f3
            0058 Unable to get descriptor
            0060 Unable to get descriptor
        */
        let gdt = [
            0x0000000000000000u64,
            0x0000000000000000,
            0x00209b0000000000,
            0x0040930000000000,
            0x00cffb000000ffff,
            0x00cff3000000ffff,
            0x0020fb0000000000,
            0x0000000000000000,
            0x71008be7b0000067,
            0x00000000fffff805,
            0x0040f30000003c00,
            0x0000000000000000,
            0x0000000000000000,
            0x0000000000000000,
        ];

        let gdtr = Gdtr {
            limit: u16::try_from(gdt.len() * 8 - 1).unwrap(),
            base: addr_of!(gdt) as u64,
        };

        let cs = Selector(0x10);
        let ss = Selector(0x18);
        let ds = Selector(0x2b);
        let tr = Selector(0x40);
        let fs = Selector(0x53);

        assert_eq!(Descriptor::try_new(&gdtr, cs).unwrap().base(), 0);
        assert_eq!(Descriptor::try_new(&gdtr, ss).unwrap().base(), 0);
        assert_eq!(Descriptor::try_new(&gdtr, ds).unwrap().base(), 0);
        assert_eq!(
            Descriptor::try_new(&gdtr, tr).unwrap().base(),
            0xfffff80571e7b000
        );
        assert_eq!(Descriptor::try_new(&gdtr, fs).unwrap().base(), 0);

        let code_segment = Descriptor::try_new(&gdtr, cs).unwrap();
        assert!(!code_segment.low64.is_16byte());
        let tss = Descriptor::try_new(&gdtr, tr).unwrap();
        assert!(tss.low64.is_16byte());
    }
}
