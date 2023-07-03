import * as React from 'react';
import { SVGProps } from 'react';

const SvgIconInfoSuccess = (props: SVGProps<SVGSVGElement>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width={18}
    height={18}
    viewBox="0 0 18 18"
    {...props}
  >
    <defs>
      <style>
        {
          '\n      .icon-info-success_svg__a{fill:#14bc6e}.icon-info-success_svg__b{fill:#fff}\n    '
        }
      </style>
    </defs>
    <g transform="translate(-786 -145)">
      <circle
        className="icon-info-success_svg__a"
        cx={9}
        cy={9}
        r={9}
        transform="translate(786 145)"
      />
      <g transform="translate(-.543)">
        <rect
          className="icon-info-success_svg__b"
          width={8}
          height={2}
          rx={1}
          transform="rotate(135 368.746 241.708)"
        />
        <rect
          className="icon-info-success_svg__b"
          width={5}
          height={2}
          rx={1}
          transform="rotate(45 212.04 1032.873)"
        />
      </g>
    </g>
  </svg>
);

export default SvgIconInfoSuccess;
