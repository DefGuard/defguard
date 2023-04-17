import * as React from 'react';
import { SVGProps } from 'react';

const SvgIconActivityWarning = (props: SVGProps<SVGSVGElement>) => (
  <svg xmlns="http://www.w3.org/2000/svg" width={16} height={16} {...props}>
    <defs>
      <style>
        {
          '\n      .icon-activity-warning_svg__a{fill:#cb3f3f}.icon-activity-warning_svg__b{clip-path:url(#icon-activity-warning_svg__a)}.icon-activity-warning_svg__c{fill:#fff}\n    '
        }
      </style>
    </defs>
    <g transform="translate(-784 -142.59)" className="icon-activity-warning_svg__b">
      <circle
        className="icon-activity-warning_svg__a"
        cx={6}
        cy={6}
        r={6}
        transform="translate(786 144.59)"
      />
      <path
        className="icon-activity-warning_svg__c"
        d="M792.722 151.843h-1.442l-.227-5.253h1.9Zm-1.726 1.639a.877.877 0 0 1 .071-.354.812.812 0 0 1 .2-.283.988.988 0 0 1 .312-.187 1.116 1.116 0 0 1 .4-.069 1.116 1.116 0 0 1 .4.069.988.988 0 0 1 .317.185.812.812 0 0 1 .2.283.877.877 0 0 1 .071.354.877.877 0 0 1-.071.354.812.812 0 0 1-.2.283.988.988 0 0 1-.312.187 1.116 1.116 0 0 1-.4.069 1.116 1.116 0 0 1-.4-.069.988.988 0 0 1-.312-.187.812.812 0 0 1-.2-.283.877.877 0 0 1-.076-.352Z"
      />
    </g>
  </svg>
);

export default SvgIconActivityWarning;
