import * as React from 'react';
import { SVGProps } from 'react';

const SvgIconTagDismiss = (props: SVGProps<SVGSVGElement>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width={16}
    height={16}
    role="img"
    {...props}
  >
    <defs>
      <clipPath id="icon-tag-dismiss_svg__a">
        <path data-name="Rectangle 2114" fill="#cbd3d8" d="M0 0h16v16H0z" />
      </clipPath>
    </defs>
    <g clipPath="url(#icon-tag-dismiss_svg__a)" fill="#cbd3d8">
      <rect
        data-name="Rectangle 2113"
        width={10}
        height={2}
        rx={1}
        transform="rotate(135 5.05 5.122)"
      />
      <rect
        data-name="Rectangle 2156"
        width={10}
        height={2}
        rx={1}
        transform="rotate(-135 7.95 3.879)"
      />
    </g>
  </svg>
);

export default SvgIconTagDismiss;
