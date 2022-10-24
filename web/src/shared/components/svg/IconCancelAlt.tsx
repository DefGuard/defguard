import * as React from 'react';
import { SVGProps } from 'react';

const SvgIconCancelAlt = (props: SVGProps<SVGSVGElement>) => (
  <svg xmlns="http://www.w3.org/2000/svg" width={22} height={22} {...props}>
    <defs>
      <clipPath id="icon-cancel-alt_svg__a">
        <path className="icon-cancel-alt_svg__a" d="M0 0h22v22H0z" />
      </clipPath>
      <style>
        {
          '\n      .icon-cancel-alt_svg__a,.icon-cancel-alt_svg__c{fill:#0c8ce0}.icon-cancel-alt_svg__a{opacity:0}.icon-cancel-alt_svg__b{clip-path:url(#icon-cancel-alt_svg__a)}\n    '
        }
      </style>
    </defs>
    <g className="icon-cancel-alt_svg__b" transform="rotate(90 11 11)">
      <rect
        className="icon-cancel-alt_svg__c"
        width={16}
        height={2}
        rx={1}
        transform="rotate(45 -2.571 9.621)"
      />
      <rect
        className="icon-cancel-alt_svg__c"
        width={16}
        height={2}
        rx={1}
        transform="rotate(135 7.429 6.621)"
      />
    </g>
  </svg>
);

export default SvgIconCancelAlt;
