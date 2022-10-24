import * as React from 'react';
import { SVGProps } from 'react';

const SvgIconHamburgerClose = (props: SVGProps<SVGSVGElement>) => (
  <svg xmlns="http://www.w3.org/2000/svg" width={30} height={30} {...props}>
    <defs>
      <clipPath id="icon-hamburger-close_svg__a">
        <path
          className="icon-hamburger-close_svg__a"
          transform="translate(-4 -4)"
          d="M0 0h30v30H0z"
        />
      </clipPath>
      <clipPath id="icon-hamburger-close_svg__b">
        <path className="icon-hamburger-close_svg__a" d="M0 0h22v22H0z" />
      </clipPath>
      <style>
        {
          '\n      .icon-hamburger-close_svg__a,.icon-hamburger-close_svg__c{fill:#899ca8}.icon-hamburger-close_svg__a{opacity:0}.icon-hamburger-close_svg__b{clip-path:url(#icon-hamburger-close_svg__a)}.icon-hamburger-close_svg__d{clip-path:url(#icon-hamburger-close_svg__b)}\n    '
        }
      </style>
    </defs>
    <g className="icon-hamburger-close_svg__b">
      <g className="icon-hamburger-close_svg__d" transform="rotate(90 11 15)">
        <rect
          className="icon-hamburger-close_svg__c"
          width={19.998}
          height={2}
          rx={1}
          transform="rotate(45 -1.57 7.208)"
        />
        <rect
          className="icon-hamburger-close_svg__c"
          width={19.997}
          height={2}
          rx={1}
          transform="rotate(135 8.429 6.208)"
        />
      </g>
    </g>
  </svg>
);

export default SvgIconHamburgerClose;
