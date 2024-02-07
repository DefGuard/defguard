import type { SVGProps } from 'react';
const SvgIconNavHamburger = (props: SVGProps<SVGSVGElement>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width={21}
    height={14}
    fill="none"
    viewBox="0 0 21 14"
    {...props}
  >
    <path
      fill="#899CA8"
      d="M19.95 0h-18a1 1 0 1 0 0 2h18a1 1 0 1 0 0-2M14 6H2a1 1 0 0 0 0 2h12a1 1 0 1 0 0-2M14 12H2a1 1 0 1 0 0 2h12a1 1 0 1 0 0-2"
    />
  </svg>
);
export default SvgIconNavHamburger;
