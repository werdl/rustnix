#include "../../../usr/src/syscall.h"

void _start() {
    const char *message = "Hello from userspace!\n";

    write(1, message, 24);

    exit(0);
}
