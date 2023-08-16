import type { SVGProps } from 'react';
const SvgIconArrowSingle2 = (props: SVGProps<SVGSVGElement>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width={22}
    height={23}
    fill="none"
    viewBox="0 0 22 23"
    {...props}
  >
    <mask
      id="icon-arrow-single-2_svg__a"
      width={22}
      height={23}
      x={0}
      y={0}
      maskUnits="userSpaceOnUse"
      style={{
        maskType: 'luminance',
      }}
    >
      <path fill="#fff" d="M22 .5H0v22h22V.5Z" />
    </mask>
    <g fill="#899CA8" mask="url(#icon-arrow-single-2_svg__a)">
      <path d="m11.878 6.55-4.243 4.243a1 1 0 1 0 1.414 1.414l4.243-4.243a1 1 0 1 0-1.414-1.414Z" />
      <path d="m7.636 12.55 4.243 4.243a1 1 0 0 0 1.414-1.414L9.05 11.136a1 1 0 0 0-1.414 1.414Z" />
    </g>
  </svg>
);
export default SvgIconArrowSingle2;
