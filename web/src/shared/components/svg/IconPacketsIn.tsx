import * as React from 'react';
import { SVGProps } from 'react';

const SvgIconPacketsIn = (props: SVGProps<SVGSVGElement>) => (
  <svg xmlns="http://www.w3.org/2000/svg" width={16} height={16} {...props}>
    <defs>
      <clipPath id="icon-packets-in_svg__a">
        <path className="icon-packets-in_svg__a" d="M0 0h16v16H0z" />
      </clipPath>
      <style>
        {
          '\n      .icon-packets-in_svg__a,.icon-packets-in_svg__c{fill:#0c8ce0}.icon-packets-in_svg__b{clip-path:url(#icon-packets-in_svg__a)}.icon-packets-in_svg__d,.icon-packets-in_svg__e{stroke:none}.icon-packets-in_svg__e{fill:#0c8ce0}\n    '
        }
      </style>
    </defs>
    <g transform="translate(.997 5)" className="icon-packets-in_svg__b">
      <rect
        className="icon-packets-in_svg__a"
        width={2}
        height={2}
        rx={1}
        transform="translate(2 2)"
      />
      <rect
        className="icon-packets-in_svg__a"
        width={2}
        height={2}
        rx={1}
        transform="translate(5 2)"
      />
      <g className="icon-packets-in_svg__c">
        <path
          className="icon-packets-in_svg__d"
          d="M9 4.234V1.766L11.056 3 9 4.234Z"
        />
        <path
          className="icon-packets-in_svg__e"
          d="M12.056 3a.991.991 0 0 1-.485.857L9.514 5.091A1 1 0 0 1 8 4.234V1.766A1 1 0 0 1 9.514.91l2.057 1.234a.991.991 0 0 1 .485.857Z"
        />
      </g>
    </g>
  </svg>
);

export default SvgIconPacketsIn;
