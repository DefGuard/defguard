import * as React from 'react';
import { SVGProps } from 'react';

const SvgIconArrowGrayLeft = (props: SVGProps<SVGSVGElement>) => (
  <svg xmlns="http://www.w3.org/2000/svg" width={22} height={22} {...props}>
    <defs>
      <clipPath id="icon-arrow-gray-left_svg__a">
        <path className="icon-arrow-gray-left_svg__a" d="M0 0h22v22H0z" />
      </clipPath>
      <style>
        {
          '\n      .icon-arrow-gray-left_svg__a,.icon-arrow-gray-left_svg__c{fill:#899ca8}.icon-arrow-gray-left_svg__a{opacity:0}.icon-arrow-gray-left_svg__b{clip-path:url(#icon-arrow-gray-left_svg__a)}\n    '
        }
      </style>
    </defs>
    <g className="icon-arrow-gray-left_svg__b">
      <rect
        className="icon-arrow-gray-left_svg__c"
        width={8}
        height={2}
        rx={1}
        transform="rotate(-45 16.742 -2.863)"
      />
      <rect
        className="icon-arrow-gray-left_svg__c"
        width={8}
        height={2}
        rx={1}
        transform="rotate(-135 9.814 5.893)"
      />
    </g>
  </svg>
);

export default SvgIconArrowGrayLeft;
