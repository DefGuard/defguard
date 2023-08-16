import type { SVGProps } from 'react';
const SvgIconExpand = (props: SVGProps<SVGSVGElement>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width={22}
    height={22}
    fill="none"
    viewBox="0 0 22 22"
    {...props}
  >
    <g fill="#0C8CE0">
      <path d="M4 13v4a1 1 0 0 0 1 1h4a1 1 0 1 0 0-2H7.413l3.657-3.657a1 1 0 0 0-1.414-1.414L6 14.585V13a1 1 0 1 0-2 0Z" />
      <path
        fillRule="evenodd"
        d="M12 5v4a1 1 0 0 0 1 1h4a1 1 0 0 0 1-1V5a1 1 0 0 0-1-1h-4a1 1 0 0 0-1 1Zm4 3V6h-2v2h2Z"
        clipRule="evenodd"
      />
    </g>
  </svg>
);
export default SvgIconExpand;
