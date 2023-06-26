import * as React from 'react';
import type { SVGProps } from 'react';
const SvgIconClip = (props: SVGProps<SVGSVGElement>) => (
  <svg xmlns="http://www.w3.org/2000/svg" width={23} height={23} fill="none" {...props}>
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
      <path fill="#fff" d="M22.5.5H.5v22h22V.5Z" />
    </mask>
    <g fill="#899CA8" mask="url(#icon-clip_svg__a)">
      <path d="M15.5 12.965h-8a1 1 0 1 0 0 2h8a1 1 0 0 0 0-2Z" />
      <path d="m16.742 12.793-2.121-2.121a1 1 0 0 0-1.414 1.414l2.121 2.121a1 1 0 0 0 1.414-1.414Z" />
      <path d="m15.328 13.671-2.121 2.122a1 1 0 1 0 1.414 1.414l2.121-2.122a1 1 0 0 0-1.414-1.414Zm-6.828.294v-8a1 1 0 0 0-2 0v8a1 1 0 1 0 2 0Z" />
    </g>
  </svg>
);
export default SvgIconClip;
