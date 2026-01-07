/**
 * Checks if mouse event clicked within any of provided rects
 */
export const detectClickInside = (event: MouseEvent, rects: DOMRect[]) => {
  for (const domRect of rects) {
    if (domRect) {
      const start_x = domRect?.x;
      const start_y = domRect?.y;
      const end_x = start_x + domRect?.width;
      const end_y = start_y + domRect.height;
      if (
        event.clientX >= start_x &&
        event.clientX <= end_x &&
        event.clientY >= start_y &&
        event.clientY <= end_y
      ) {
        return true;
      }
    }
  }
  return false;
};
