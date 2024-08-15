import type { SVGProps } from 'react';
const SvgIconCancel = (props: SVGProps<SVGSVGElement>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width={12}
    height={12}
    fill="none"
    viewBox="0 0 12 12"
    {...props}
  >
    <path
      fill="#C4C4C4"
      d="m10.628 9.471-8.1-8.1A.818.818 0 1 0 1.371 2.53l8.1 8.1a.818.818 0 0 0 1.157-1.158"
    />
    <path
      fill="#C4C4C4"
      d="m2.529 10.628 8.1-8.1A.818.818 0 0 0 9.47 1.373l-8.1 8.1a.818.818 0 1 0 1.158 1.156"
    />
  </svg>
);
export default SvgIconCancel;
