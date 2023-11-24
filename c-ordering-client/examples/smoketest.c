#include <dlfcn.h>
#include <stdio.h>

// int add(int a, int b);
typedef int (*add_t)(int, int);

int main(int argc, char **argv) {
  void* handle = dlopen("../../target/debug/libc_ordering_client.so", RTLD_LAZY);
  if (!handle) {
    fprintf(stderr, "%s\n", dlerror());
    return 1;
  }
  add_t add = dlsym(handle, "add");
  printf("3 + 4 = %d\n", add(3, 4));
}
