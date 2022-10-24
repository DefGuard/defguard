import * as React from 'react';
import { SVGProps } from 'react';

const SvgIconConnected = (props: SVGProps<SVGSVGElement>) => (
  <svg xmlns="http://www.w3.org/2000/svg" width={16} height={16} {...props}>
    <defs>
      <style>
        {
          '\n      .icon-connected_svg__a{fill:#cb3f3f}.icon-connected_svg__b{clip-path:url(#icon-connected_svg__a)}.icon-connected_svg__c,.icon-connected_svg__e{fill:none}.icon-connected_svg__c{stroke:#14bc6e;stroke-width:2px}.icon-connected_svg__d{stroke:none}\n    '
        }
      </style>
    </defs>
    <g className="icon-connected_svg__b">
      <g className="icon-connected_svg__c" transform="translate(4 4)">
        <circle className="icon-connected_svg__d" cx={4} cy={4} r={4} />
        <circle className="icon-connected_svg__e" cx={4} cy={4} r={3} />
      </g>
    </g>
  </svg>
);

export default SvgIconConnected;
