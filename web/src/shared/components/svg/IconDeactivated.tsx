import * as React from 'react';
import { SVGProps } from 'react';

const SvgIconDeactivated = (props: SVGProps<SVGSVGElement>) => (
  <svg xmlns="http://www.w3.org/2000/svg" width={22} height={22} {...props}>
    <defs>
      <clipPath id="icon-deactivated_svg__a">
        <path className="icon-deactivated_svg__a" d="M0 0h22v22H0z" />
      </clipPath>
      <style>
        {
          '\n      .icon-deactivated_svg__a,.icon-deactivated_svg__d{fill:#899ca8}.icon-deactivated_svg__a{opacity:0}.icon-deactivated_svg__b{clip-path:url(#icon-deactivated_svg__a)}.icon-deactivated_svg__c,.icon-deactivated_svg__f{fill:none}.icon-deactivated_svg__c{stroke:#899ca8;stroke-width:2px}.icon-deactivated_svg__e{stroke:none}\n    '
        }
      </style>
    </defs>
    <g className="icon-deactivated_svg__b" transform="rotate(90 11 11)">
      <g className="icon-deactivated_svg__c" transform="translate(3 3)">
        <circle className="icon-deactivated_svg__e" cx={8} cy={8} r={8} />
        <circle className="icon-deactivated_svg__f" cx={8} cy={8} r={7} />
      </g>
      <rect
        className="icon-deactivated_svg__d"
        width={14}
        height={2}
        rx={1}
        transform="rotate(45 -3.07 10.83)"
      />
    </g>
  </svg>
);

export default SvgIconDeactivated;
