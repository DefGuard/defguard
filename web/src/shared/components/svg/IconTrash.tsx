import * as React from 'react';
import { SVGProps } from 'react';
const SvgIconTrash = (props: SVGProps<SVGSVGElement>) => (
  <svg xmlns="http://www.w3.org/2000/svg" width={22} height={22} role="img" {...props}>
    <defs>
      <clipPath id="icon-trash_svg__a">
        <path
          data-name="Rectangle 2961"
          transform="rotate(90 -429 1032)"
          fill="#899ca8"
          opacity={0}
          d="M0 0h22v22H0z"
        />
      </clipPath>
    </defs>
    <g transform="translate(-581 -1461)" clipPath="url(#icon-trash_svg__a)">
      <g data-name="Group 4725" fill="#899ca8">
        <path
          data-name="Path 5669"
          d="M597.996 1467.459a1 1 0 0 0-1.07.924l-.835 11.376h-7.9l-.835-11.376a1 1 0 1 0-1.994.147l.9 12.3a1 1 0 0 0 1 .927h9.755a1 1 0 0 0 1-.927l.9-12.3a1 1 0 0 0-.921-1.071Z"
        />
        <path
          data-name="Path 5670"
          d="M599.285 1465.138h-3.546l-.846-2.463a1 1 0 0 0-.945-.675h-4.01a1 1 0 0 0-.945.675l-.846 2.463H585a1 1 0 0 0 0 2h14.285a1 1 0 0 0 0-2Zm-8.633-1.138h2.582l.391 1.138h-3.364Z"
        />
      </g>
    </g>
  </svg>
);
export default SvgIconTrash;
