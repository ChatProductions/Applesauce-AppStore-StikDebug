#include <SDL.h>
/* Display/input smoke test only. WebKit integration has NOT begun. */
int main(int argc, char **argv) {
  (void)argc; (void)argv;
  SDL_SetHint(SDL_HINT_TOUCH_MOUSE_EVENTS, "0");
  if (SDL_Init(SDL_INIT_VIDEO | SDL_INIT_GAMECONTROLLER) != 0) return 1;
  SDL_Window *win = SDL_CreateWindow("VitaSurf POC", SDL_WINDOWPOS_UNDEFINED, SDL_WINDOWPOS_UNDEFINED, 960, 544, 0);
  if (!win) { SDL_Quit(); return 2; }
  SDL_Renderer *ren = SDL_CreateRenderer(win, -1, SDL_RENDERER_ACCELERATED);
  if (!ren) ren = SDL_CreateRenderer(win, -1, SDL_RENDERER_SOFTWARE);
  if (!ren) { SDL_DestroyWindow(win); SDL_Quit(); return 3; }
  SDL_GameController *pad = NULL;
  if (SDL_NumJoysticks() > 0 && SDL_IsGameController(0)) pad = SDL_GameControllerOpen(0);
  int running = 1;
  while (running) {
    SDL_Event e;
    while (SDL_PollEvent(&e)) {
      if (e.type == SDL_QUIT) running = 0;
      if (e.type == SDL_CONTROLLERBUTTONDOWN && e.cbutton.button == SDL_CONTROLLER_BUTTON_B) running = 0;
    }
    SDL_SetRenderDrawColor(ren, 17, 24, 39, 255);
    SDL_RenderClear(ren);
    SDL_Rect header = {0, 0, 960, 72};
    SDL_SetRenderDrawColor(ren, 34, 48, 71, 255);
    SDL_RenderFillRect(ren, &header);
    SDL_Rect address = {22, 16, 916, 40};
    SDL_SetRenderDrawColor(ren, 220, 231, 241, 255);
    SDL_RenderFillRect(ren, &address);
    SDL_RenderPresent(ren);
    SDL_Delay(16);
  }
  if (pad) SDL_GameControllerClose(pad);
  SDL_DestroyRenderer(ren); SDL_DestroyWindow(win); SDL_Quit();
  return 0;
}
