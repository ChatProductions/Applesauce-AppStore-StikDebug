#include <psp2/kernel/threadmgr.h>
/* Minimal VitaSDK packaging baseline. No browser engine or graphics yet. */
int main(void) {
  sceKernelDelayThread(1000000);
  return 0;
}
