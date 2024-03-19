import type { SVGProps } from 'react';
const SvgIconCollapse = (props: SVGProps<SVGSVGElement>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width={23}
    height={22}
    fill="none"
    viewBox="0 0 23 22"
    {...props}
  >
    <g fill="#0C8CE0">
      <path d="M18.5 9V5a1 1 0 0 0-1-1h-4a1 1 0 1 0 0 2h1.587L14.06 7.026a1 1 0 0 0 1.414 1.414L16.5 7.415V9a1 1 0 1 0 2 0" />
      <path
        fillRule="evenodd"
        d="M6.5 16h5.234v-5.234H6.5zm5.234 2a2 2 0 0 0 2-2v-5.234a2 2 0 0 0-2-2H6.5a2 2 0 0 0-2 2V16a2 2 0 0 0 2 2z"
        clipRule="evenodd"
      />
    </g>
  </svg>
);
export default SvgIconCollapse;
