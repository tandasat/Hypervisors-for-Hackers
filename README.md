# Hypervisors-for-Hackers

The materials of the "Hypervisors for Hackers: Security from the Hardware Up" class held at
[Global Cybersecurity Camp 2026 Vietnam](https://gcc.ac/#Hypervisors-for-Hackers). This repository
contains a minimalistic hypervisor for Windows on Intel processors and lecture materials.

## Course format

The class materials are designed for an interactive classroom setting and less effective for
self-learning due to light explanations. We decided to publish this as it would still be useful to
some, however. If you are interested in the interactive class with the author, please check out the
schedule of the next public class at [System Programming Lab](https://tandasat.github.io/).

## Directory structure
- 📖[lecture/](lecture/) for the class materials.
- 🦀[src/](src/) for source code of the hypervisor.

## Supported platforms

A Windows host with the Intel processor is required. For the detailed setup instructions, see
[lecture/setup/README.md](lecture/setup/README.md).

## Prerequisite

Some familiarity with the x86-64 architecture is a prerequisite for this lecture. If you want to review relevant topics, I recommend checking out the [Architecture 2001: x86-64 OS Internals](https://p.ost2.fyi/courses/course-v1:OpenSecurityTraining2+Arch2001_x86-64_OS_Internals+2021_v1/about) course at OST2. It is a free, high-quality online learning material. The following are the particularly relevant parts to follow in this lecture:

- CPUID
    - cpuid: CPU Feature Identification instruction
- Processor Execution Modes
    - Processor Execution Modes
- Model-Specific Registers (MSRs)
    - Reading and writing MSRs (rdmsr, wrmsr)
- Privilege Rings & Segmentation
    - Privilege Rings Start
    - Segmentation & Segment Registers
    - Global Descriptor Table (GDT) & Local Descriptor Table (LDT)
- Interrupts, Incomplete section
    - Interrupts vs. Exceptions
