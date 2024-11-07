#if defined(__aarch64__) && !defined(__arm64__)
#define __arm64__ 1
#endif
#include <stddef.h>
#include <stdint.h>
#include <sys/statvfs.h>
#include <sys/time.h>
#include <smb2/smb2.h>
#include <smb2/libsmb2.h>
#include <smb2/libsmb2-raw.h>
