import * as React from 'react';
import { SVGProps } from 'react';

const SvgIconCheckmarkWhiteBig = (props: SVGProps<SVGSVGElement>) => (
  <svg xmlns="http://www.w3.org/2000/svg" width={36} height={36} {...props}>
    <defs>
      <clipPath id="icon-checkmark-white-big_svg__a">
        <path
          className="icon-checkmark-white-big_svg__a"
          transform="translate(.203 .203)"
          d="M0 0h36v36H0z"
        />
      </clipPath>
      <style>
        {
          '\n      .icon-checkmark-white-big_svg__a,.icon-checkmark-white-big_svg__c{fill:#fff}.icon-checkmark-white-big_svg__a{opacity:0}.icon-checkmark-white-big_svg__b{clip-path:url(#icon-checkmark-white-big_svg__a)}\n    '
        }
      </style>
    </defs>
    <g
      className="icon-checkmark-white-big_svg__b"
      transform="rotate(90 18.203 18)"
    >
      <rect
        className="icon-checkmark-white-big_svg__c"
        width={19.857}
        height={3.31}
        rx={1.655}
        transform="rotate(45 -.06 17.375)"
      />
      <rect
        className="icon-checkmark-white-big_svg__c"
        width={13.238}
        height={3.31}
        rx={1.655}
        transform="rotate(-45 39.078 -4.59)"
      />
    </g>
  </svg>
);

export default SvgIconCheckmarkWhiteBig;
