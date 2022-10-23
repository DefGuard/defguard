import * as React from 'react';
import { SVGProps } from 'react';

const SvgIconActivityRemoved = (props: SVGProps<SVGSVGElement>) => (
  <svg xmlns="http://www.w3.org/2000/svg" width={16} height={16} {...props}>
    <defs>
      <style>
        {
          '\n      .icon-activity-removed_svg__a{fill:#cb3f3f}.icon-activity-removed_svg__b{clip-path:url(#icon-activity-removed_svg__a)}.icon-activity-removed_svg__c{fill:none}.icon-activity-removed_svg__d,.icon-activity-removed_svg__e{stroke:none}.icon-activity-removed_svg__e{fill:#cb3f3f}\n    '
        }
      </style>
    </defs>
    <g className="icon-activity-removed_svg__b">
      <g className="icon-activity-removed_svg__c">
        <path
          className="icon-activity-removed_svg__d"
          d="M14 8a6 6 0 1 1-6-6 6 6 0 0 1 6 6Z"
        />
        <path
          className="icon-activity-removed_svg__e"
          d="M12 8c0-2.206-1.794-4-4-4S4 5.794 4 8s1.794 4 4 4 4-1.794 4-4m2 0A6 6 0 1 1 2 8a6 6 0 0 1 12 0Z"
        />
      </g>
      <rect
        className="icon-activity-removed_svg__a"
        width={10}
        height={2}
        rx={1}
        transform="rotate(135 5.05 5.122)"
      />
    </g>
  </svg>
);

export default SvgIconActivityRemoved;
