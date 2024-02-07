import type { SVGProps } from 'react';
const SvgIconCheckmark = (props: SVGProps<SVGSVGElement>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width={12}
    height={9}
    fill="none"
    viewBox="0 0 12 9"
    {...props}
  >
    <path
      fill="#fff"
      d="m5.418 7.828 5.85-5.85a.827.827 0 1 0-1.17-1.17l-5.85 5.85a.827.827 0 1 0 1.17 1.17"
    />
    <path
      fill="#fff"
      d="M5.37 6.7 1.86 3.19A.827.827 0 1 0 .69 4.36L4.2 7.87A.827.827 0 1 0 5.37 6.7"
    />
  </svg>
);
export default SvgIconCheckmark;
