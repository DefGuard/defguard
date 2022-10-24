import * as React from 'react';
import { SVGProps } from 'react';

const SvgIconHamburgerMenu = (props: SVGProps<SVGSVGElement>) => (
  <svg xmlns="http://www.w3.org/2000/svg" width={30} height={30} {...props}>
    <defs>
      <style>
        {
          '\n      .icon-hamburger-menu_svg__a,.icon-hamburger-menu_svg__c{fill:#899ca8}.icon-hamburger-menu_svg__a{opacity:0}\n    '
        }
      </style>
    </defs>
    <g transform="translate(-308 -223)" className="icon-hamburger-menu_svg__b">
      <rect
        className="icon-hamburger-menu_svg__c"
        width={20}
        height={2}
        rx={1}
        transform="translate(313 231)"
      />
      <rect
        className="icon-hamburger-menu_svg__c"
        width={14}
        height={2}
        rx={1}
        transform="translate(313 237)"
      />
      <rect
        className="icon-hamburger-menu_svg__c"
        width={14}
        height={2}
        rx={1}
        transform="translate(313 243)"
      />
    </g>
  </svg>
);

export default SvgIconHamburgerMenu;
