import * as React from 'react';
import { SVGProps } from 'react';

const SvgIconUserListElement = (props: SVGProps<SVGSVGElement>) => (
  <svg xmlns="http://www.w3.org/2000/svg" width={22} height={22} {...props}>
    <defs>
      <style>
        {
          '\n      .icon-user-list-element_svg__a,.icon-user-list-element_svg__c{fill:#899ca8}.icon-user-list-element_svg__a{opacity:0}.icon-user-list-element_svg__b{clip-path:url(#icon-user-list-element_svg__a)}\n    '
        }
      </style>
    </defs>
    <g
      transform="translate(-313 -224.475)"
      className="icon-user-list-element_svg__b"
    >
      <rect
        className="icon-user-list-element_svg__c"
        width={10}
        height={2}
        rx={1}
        transform="translate(319 237.475)"
      />
      <rect
        className="icon-user-list-element_svg__c"
        width={5}
        height={2}
        rx={1}
        transform="rotate(45 -119.83 511.254)"
      />
      <rect
        className="icon-user-list-element_svg__c"
        width={5}
        height={2}
        rx={1}
        transform="rotate(-45 453.425 -271.805)"
      />
      <rect
        className="icon-user-list-element_svg__c"
        width={10}
        height={2}
        rx={1}
        transform="rotate(90 45.762 275.238)"
      />
    </g>
  </svg>
);

export default SvgIconUserListElement;
