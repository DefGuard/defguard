import * as React from 'react';
import { SVGProps } from 'react';

const SvgIconActivityAdd = (props: SVGProps<SVGSVGElement>) => (
  <svg xmlns="http://www.w3.org/2000/svg" width={16} height={16} {...props}>
    <defs>
      <style>
        {
          '\n      .icon-activity-add_svg__a{fill:#cb3f3f}.icon-activity-add_svg__b{clip-path:url(#icon-activity-add_svg__a)}.icon-activity-add_svg__c{fill:#0c8ce0}\n    '
        }
      </style>
    </defs>
    <g className="icon-activity-add_svg__b">
      <rect
        className="icon-activity-add_svg__c"
        width={10}
        height={2}
        rx={1}
        transform="rotate(-90 10 3)"
      />
      <rect
        className="icon-activity-add_svg__c"
        width={10}
        height={2}
        rx={1}
        transform="rotate(-180 6.5 4.5)"
      />
    </g>
  </svg>
);

export default SvgIconActivityAdd;
