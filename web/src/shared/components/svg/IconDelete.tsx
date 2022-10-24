import * as React from 'react';
import { SVGProps } from 'react';

const SvgIconDelete = (props: SVGProps<SVGSVGElement>) => (
  <svg xmlns="http://www.w3.org/2000/svg" width={22} height={22} {...props}>
    <defs>
      <clipPath id="icon-delete_svg__a">
        <path className="icon-delete_svg__a" d="M0 0h22v22H0z" />
      </clipPath>
      <style>
        {
          '\n      .icon-delete_svg__a,.icon-delete_svg__d{fill:#cb3f3f}.icon-delete_svg__a{opacity:0}.icon-delete_svg__b{clip-path:url(#icon-delete_svg__a)}.icon-delete_svg__c,.icon-delete_svg__f{fill:none}.icon-delete_svg__c{stroke:#cb3f3f;stroke-width:2px}.icon-delete_svg__e{stroke:none}\n    '
        }
      </style>
    </defs>
    <g className="icon-delete_svg__b" transform="rotate(90 11 11)">
      <g className="icon-delete_svg__c" transform="translate(3 3)">
        <circle className="icon-delete_svg__e" cx={8} cy={8} r={8} />
        <circle className="icon-delete_svg__f" cx={8} cy={8} r={7} />
      </g>
      <rect
        className="icon-delete_svg__d"
        width={8}
        height={2}
        rx={1}
        transform="rotate(45 -4.57 14.45)"
      />
      <rect
        className="icon-delete_svg__d"
        width={8}
        height={2}
        rx={1}
        transform="rotate(135 5.43 7.45)"
      />
    </g>
  </svg>
);

export default SvgIconDelete;
