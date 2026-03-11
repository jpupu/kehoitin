#include <string.h>
#include <stdlib.h>

int gethostname(char *name, size_t len) {
    const char* fake_name = getenv("FAKEHOSTNAME");
    if (len > strlen(fake_name)) {
        strcpy(name, fake_name);
        return 0;
    }
    return -1;
}
