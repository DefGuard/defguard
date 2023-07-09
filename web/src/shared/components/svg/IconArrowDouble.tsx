import type { SVGProps } from 'react';
import * as React from 'react';
const SvgIconArrowDouble = (props: SVGProps<SVGSVGElement>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width={22}
    height={22}
    viewBox="0 0 22 22"
    fill="none"
    {...props}
  >
    <g fill="#899CA8">
      <path d="m11.708 7.465 4.243 4.243a1 1 0 0 0 1.414-1.414L13.122 6.05a1 1 0 1 0-1.414 1.414Zm-6 0 4.243 4.243a1 1 0 0 0 1.414-1.414L7.122 6.05a1 1 0 1 0-1.414 1.414Z" />
      <path d="m15.95 10.636-4.243 4.243a1 1 0 0 0 1.414 1.414l4.243-4.243a1 1 0 0 0-1.414-1.414Zm-6 0L5.707 14.88a1 1 0 1 0 1.414 1.414l4.243-4.243a1 1 0 0 0-1.414-1.414Z" />
    </g>
  </svg>
);
export default SvgIconArrowDouble;
