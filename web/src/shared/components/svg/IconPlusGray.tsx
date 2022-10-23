import * as React from 'react';
import { SVGProps } from 'react';

const SvgIconPlusGray = (props: SVGProps<SVGSVGElement>) => (
  <svg xmlns="http://www.w3.org/2000/svg" width={22} height={22} {...props}>
    <defs>
      <clipPath id="icon-plus-gray_svg__a">
        <path className="icon-plus-gray_svg__a" d="M0 0h22v22H0z" />
      </clipPath>
      <style>
        {
          '\n      .icon-plus-gray_svg__a,.icon-plus-gray_svg__c{fill:#899ca8}.icon-plus-gray_svg__a{opacity:0}.icon-plus-gray_svg__b{clip-path:url(#icon-plus-gray_svg__a)}\n    '
        }
      </style>
    </defs>
    <g className="icon-plus-gray_svg__b">
      <rect
        className="icon-plus-gray_svg__c"
        width={10}
        height={2}
        rx={1}
        transform="rotate(-90 13 3)"
      />
      <rect
        className="icon-plus-gray_svg__c"
        width={10}
        height={2}
        rx={1}
        transform="rotate(-180 8 6)"
      />
    </g>
  </svg>
);

export default SvgIconPlusGray;
