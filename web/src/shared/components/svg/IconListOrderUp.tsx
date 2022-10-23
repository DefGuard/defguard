import * as React from 'react';
import { SVGProps } from 'react';

const SvgIconListOrderUp = (props: SVGProps<SVGSVGElement>) => (
  <svg xmlns="http://www.w3.org/2000/svg" width={22} height={22} {...props}>
    <defs>
      <clipPath id="icon-list-order-up_svg__a">
        <path className="icon-list-order-up_svg__a" d="M0 0h22v22H0z" />
      </clipPath>
      <style>
        {
          '\n      .icon-list-order-up_svg__a,.icon-list-order-up_svg__c,.icon-list-order-up_svg__d{fill:#899ca8}.icon-list-order-up_svg__a{opacity:0}.icon-list-order-up_svg__b{clip-path:url(#icon-list-order-up_svg__a)}.icon-list-order-up_svg__e,.icon-list-order-up_svg__f{stroke:none}.icon-list-order-up_svg__f{fill:#899ca8}\n    '
        }
      </style>
    </defs>
    <g className="icon-list-order-up_svg__b">
      <g transform="translate(8 6)">
        <rect
          className="icon-list-order-up_svg__c"
          width={14}
          height={2}
          rx={1}
          transform="translate(0 8)"
        />
        <rect
          className="icon-list-order-up_svg__c"
          width={10}
          height={2}
          rx={1}
          transform="translate(0 4)"
        />
        <rect
          className="icon-list-order-up_svg__c"
          width={10}
          height={2}
          rx={1}
        />
      </g>
      <g transform="rotate(-90 8 8)">
        <rect
          className="icon-list-order-up_svg__c"
          width={8}
          height={2}
          rx={1}
          transform="translate(0 2)"
        />
        <g className="icon-list-order-up_svg__d">
          <path
            className="icon-list-order-up_svg__e"
            d="M7 4.234V1.766L9.056 3 7 4.234Z"
          />
          <path
            className="icon-list-order-up_svg__f"
            d="M10.056 3a.991.991 0 0 1-.485.857L7.514 5.091A1 1 0 0 1 6 4.234V1.766A1 1 0 0 1 7.514.91l2.057 1.234a.991.991 0 0 1 .485.857Z"
          />
        </g>
      </g>
    </g>
  </svg>
);

export default SvgIconListOrderUp;
