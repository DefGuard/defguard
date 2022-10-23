import * as React from 'react';
import { SVGProps } from 'react';

const SvgIconDisconnected = (props: SVGProps<SVGSVGElement>) => (
  <svg xmlns="http://www.w3.org/2000/svg" width={16} height={16} {...props}>
    <defs>
      <style>
        {
          '\n      .icon-disconnected_svg__a{fill:#cb3f3f}.icon-disconnected_svg__b{clip-path:url(#icon-disconnected_svg__a)}.icon-disconnected_svg__c{fill:#899ca8}\n    '
        }
      </style>
    </defs>
    <g className="icon-disconnected_svg__b">
      <circle
        className="icon-disconnected_svg__c"
        cx={3}
        cy={3}
        r={3}
        transform="translate(5 5)"
      />
    </g>
  </svg>
);

export default SvgIconDisconnected;
