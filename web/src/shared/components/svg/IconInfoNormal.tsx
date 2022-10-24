import * as React from 'react';
import { SVGProps } from 'react';

const SvgIconInfoNormal = (props: SVGProps<SVGSVGElement>) => (
  <svg xmlns="http://www.w3.org/2000/svg" width={18} height={18} {...props}>
    <defs>
      <style>
        {
          '\n      .icon-info-normal_svg__icon-info-normal-a{fill:#899ca8}.icon-info-normal_svg__icon-info-normal-b{fill:#fff}\n    '
        }
      </style>
    </defs>
    <g transform="translate(-786 -145)">
      <circle
        className="icon-info-normal_svg__icon-info-normal-a"
        cx={9}
        cy={9}
        r={9}
        transform="translate(786 145)"
      />
      <rect
        className="icon-info-normal_svg__icon-info-normal-b"
        width={2}
        height={7}
        rx={1}
        transform="translate(794 152)"
      />
      <rect
        className="icon-info-normal_svg__icon-info-normal-b"
        width={2}
        height={2}
        rx={1}
        transform="translate(794 149)"
      />
    </g>
  </svg>
);

export default SvgIconInfoNormal;
