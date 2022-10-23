import * as React from 'react';
import { SVGProps } from 'react';

const SvgIconAsterix = (props: SVGProps<SVGSVGElement>) => (
  <svg xmlns="http://www.w3.org/2000/svg" width={22} height={22} {...props}>
    <defs>
      <clipPath id="icon-asterix_svg__a">
        <path className="icon-asterix_svg__a" d="M0 0h22v22H0z" />
      </clipPath>
      <style>
        {
          '\n      .icon-asterix_svg__a,.icon-asterix_svg__c{fill:#cbd3d8}.icon-asterix_svg__a{opacity:0}.icon-asterix_svg__b{clip-path:url(#icon-asterix_svg__a)}\n    '
        }
      </style>
    </defs>
    <g className="icon-asterix_svg__b" transform="rotate(90 11 11)">
      <rect
        className="icon-asterix_svg__c"
        width={12}
        height={2}
        rx={1}
        transform="rotate(60 -.16 10.33)"
      />
      <rect
        className="icon-asterix_svg__c"
        width={12}
        height={2}
        rx={1}
        transform="rotate(-60 17.16 1.67)"
      />
      <rect
        className="icon-asterix_svg__c"
        width={12}
        height={2}
        rx={1}
        transform="rotate(180 8.5 6)"
      />
    </g>
  </svg>
);

export default SvgIconAsterix;
