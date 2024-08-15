import type { SVGProps } from 'react';
const SvgIconClip = (props: SVGProps<SVGSVGElement>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width={23}
    height={23}
    fill="none"
    viewBox="0 0 23 23"
    {...props}
  >
    <mask
      id="icon-clip_svg__a"
      width={23}
      height={23}
      x={0}
      y={0}
      maskUnits="userSpaceOnUse"
      style={{
        maskType: 'luminance',
      }}
    >
      <path fill="#fff" d="M22.5.5H.5v22h22z" />
    </mask>
    <g fill="#899CA8" mask="url(#icon-clip_svg__a)">
      <path d="M15.5 12.965h-8a1 1 0 1 0 0 2h8a1 1 0 0 0 0-2" />
      <path d="m16.742 12.793-2.121-2.121a1 1 0 0 0-1.414 1.414l2.121 2.121a1 1 0 0 0 1.414-1.414" />
      <path d="m15.328 13.671-2.121 2.122a1 1 0 1 0 1.414 1.414l2.121-2.122a1 1 0 0 0-1.414-1.414M8.5 13.965v-8a1 1 0 0 0-2 0v8a1 1 0 1 0 2 0" />
    </g>
  </svg>
);
export default SvgIconClip;
