# Minivisor

A minimalisic hypervisor for Windows on Intel processors.

## Building and testing

See [../lecture/setup/README.md](../lecture/setup/README.md).

## Directory structure

- [hvcore/](hvcore/) - The OS agnostic parts of the hypervisor, ie, the core. This code, in particular
  [hvcore/src/host.rs](hvcore/src/host.rs) is where you should look into.
- [minivisor/](minivisor/) - The Windows specific parts of the hypervisor. The module entry point is
  in [minivisor/src/lib.rs](minivisor/src/lib.rs).
- [xtask](xtask/) - The build and test assist host program.

## Goal

The top most focus of this hypervisor is simplicity. The project aims to be minimal to able to
virtualize the currently running Windows 11 on VMware. The author believes it is the most beneficial
for the entry-level leaners to have something playable, that is, a small sample project they can
read, modify and test at their disposal. Thus, this project is to be used for your starting point of
learning and not as a robust reference implementation.

## References

All references to external resources (denoted with "See:" as source code comment) refers to
[Intel 64 and IA-32 Architectures Software Developer’s Manual](https://www.intel.com/sdm/)
Revision 89 (October 2025) unless otherwise stated.
