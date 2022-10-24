import * as React from 'react';
import { SVGProps } from 'react';

const SvgIconPacketsOut = (props: SVGProps<SVGSVGElement>) => (
  <svg xmlns="http://www.w3.org/2000/svg" width={16} height={16} {...props}>
    <defs>
      <clipPath id="icon-packets-out_svg__a">
        <path className="icon-packets-out_svg__a" d="M0 0h16v16H0z" />
      </clipPath>
      <style>
        {
          '\n      .icon-packets-out_svg__a,.icon-packets-out_svg__c{fill:#cb3f3f}.icon-packets-out_svg__b{clip-path:url(#icon-packets-out_svg__a)}.icon-packets-out_svg__d,.icon-packets-out_svg__e{stroke:none}.icon-packets-out_svg__e{fill:#cb3f3f}\n    '
        }
      </style>
    </defs>
    <g transform="translate(1.997 5)" className="icon-packets-out_svg__b">
      <rect
        className="icon-packets-out_svg__a"
        width={2}
        height={2}
        rx={1}
        transform="translate(9 2)"
      />
      <rect
        className="icon-packets-out_svg__a"
        width={2}
        height={2}
        rx={1}
        transform="translate(6 2)"
      />
      <g className="icon-packets-out_svg__c">
        <path
          className="icon-packets-out_svg__d"
          d="M4 1.766v2.468L1.944 3 4 1.766Z"
        />
        <path
          className="icon-packets-out_svg__e"
          d="M.944 3c0-.332.161-.663.485-.857L3.486.909A1 1 0 0 1 5 1.766v2.468a1 1 0 0 1-1.514.857L1.429 3.857A.991.991 0 0 1 .944 3Z"
        />
      </g>
    </g>
  </svg>
);

export default SvgIconPacketsOut;
