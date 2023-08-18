import type { SVGProps } from 'react';
const SvgSubtract = (props: SVGProps<SVGSVGElement>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width={8}
    height={8}
    fill="none"
    viewBox="0 0 8 8"
    {...props}
  >
    <path
      fill="#B88F30"
      fillRule="evenodd"
      d="M4 8a4 4 0 1 0 0-8 4 4 0 0 0 0 8Zm0-2a2 2 0 1 0 0-4 2 2 0 0 0 0 4Z"
      clipRule="evenodd"
    />
  </svg>
);
export default SvgSubtract;
