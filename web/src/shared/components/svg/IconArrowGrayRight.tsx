import * as React from 'react';
import { SVGProps } from 'react';

const SvgIconArrowGrayRight = (props: SVGProps<SVGSVGElement>) => (
  <svg xmlns="http://www.w3.org/2000/svg" width={22} height={22} {...props}>
    <defs>
      <clipPath id="icon-arrow-gray-right_svg__a">
        <path className="icon-arrow-gray-right_svg__a" d="M0 0h22v22H0z" />
      </clipPath>
      <style>
        {
          '\n      .icon-arrow-gray-right_svg__a,.icon-arrow-gray-right_svg__c{fill:#899ca8}.icon-arrow-gray-right_svg__a{opacity:0}.icon-arrow-gray-right_svg__b{clip-path:url(#icon-arrow-gray-right_svg__a)}\n    '
        }
      </style>
    </defs>
    <g className="icon-arrow-gray-right_svg__b">
      <rect
        className="icon-arrow-gray-right_svg__c"
        width={8}
        height={2}
        rx={1}
        transform="rotate(-135 9.4 3.379)"
      />
      <rect
        className="icon-arrow-gray-right_svg__c"
        width={8}
        height={2}
        rx={1}
        transform="rotate(-45 22.814 -1.864)"
      />
    </g>
  </svg>
);

export default SvgIconArrowGrayRight;
