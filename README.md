# rustnix
> another useless toy Unix clone

![Screenshot of rustnix](img/rustnix.jpg)

![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/werdl/rustnix/test.yml)
![GitHub last commit](https://img.shields.io/github/last-commit/werdl/rustnix)
![GitHub top language](https://img.shields.io/github/languages/top/werdl/rustnix)

## Features
- [x] Interrupts
- [x] Memory allocation
- [x] ATA disk driver
- [x] Basic inode-based filesystem
- [x] Clock module
- [x] Basic async/await support
- [x] Syscalls
- [ ] Processes
- [ ] ELF Binaries
- [ ] Basic Userspace

## Thanks
### Code
- for the initial stages of this project, I followed [https://os.phil-opp.com/](https://os.phil-opp.com/)
- the ATA driver code was taken mainly from [https://github.com/vinc/moros](https://github.com/vinc/moros)
- I have read some of the code of [moros](https://github.com/vinc/moros) and [RedoxOS](https://www.redox-os.org/) for inspiration and implementation details
- the [OSDev Wiki](https://wiki.osdev.org) has also been helpful
### Libraries used
- `acpi`
- `aml`
- `bit_field`
- `bootloader`
- `chrono`
- `hashbrown`
- `lazy_static`
- `linked_list_allocator`
- `log`
- `pc-keyboard`
- `pic8259`
- `spin`
- `typenum`
- `uart_16550`
- `volatile` (several versions behind)
- `x86_64`

## Syscalls
|Number|Name|Arg1|Arg2|Arg3|Arg4|Return|
|------|----|----|----|----|----|------|
|1|`read`|`fd`|`buf` (ptr)|`buf_len`||`nread`|
|2|`write`|`fd`|`buf` (ptr)|`buf_len`||`nwritten`|
|3|`open`|`path` (ptr)|`flags`|`mode` (bitfield)||`fd`|
|4|`close`|`fd`||||0 or -1 (err)|
|5|`flush`|`fd`||||0 or -1 (err)|
|7|`sleep`|`nanoseconds`||||0|
|8|`wait` (uses TSC)|`nanoseconds`||||0|
|13|`stop`|`kind` (0=shutdown, 1=reboot)|||||
|18|`alloc`|`size`|`align`|||`ptr`|
|19|`free`|`ptr`|`size`|`align`|||
|20|`geterrno`|||||`errno`|
|21|`poll`|`fd`|`event` (1=read, 2=write)|||`ready`|
