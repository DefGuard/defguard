import type { SVGProps } from 'react';
const SvgIconX = (props: SVGProps<SVGSVGElement>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width={22}
    height={22}
    fill="none"
    viewBox="0 0 22 22"
    {...props}
  >
    <g opacity={0.8}>
      <mask
        id="icon-X_svg__a"
        width={22}
        height={22}
        x={0}
        y={0}
        maskUnits="userSpaceOnUse"
        style={{
          maskType: 'luminance',
        }}
      >
        <path fill="#fff" d="M22 22V0H0v22h22Z" />
      </mask>
      <g fill="#899CA8" mask="url(#icon-X_svg__a)">
        <path d="m6.757 16.657 9.9-9.9a1 1 0 0 0-1.414-1.414l-9.9 9.9a1 1 0 1 0 1.414 1.414Z" />
        <path d="m5.343 6.757 9.9 9.9a1 1 0 0 0 1.414-1.414l-9.9-9.9a1 1 0 1 0-1.414 1.414Z" />
      </g>
    </g>
  </svg>
);
export default SvgIconX;
