import * as React from 'react';
import { SVGProps } from 'react';

const SvgIconCopy = (props: SVGProps<SVGSVGElement>) => (
  <svg xmlns="http://www.w3.org/2000/svg" width={22} height={22} {...props}>
    <defs>
      <clipPath id="icon-copy_svg__a">
        <path
          data-name="Rectangle 2627"
          fill="#899ca8"
          opacity={0}
          d="M0 0h22v22H0z"
        />
      </clipPath>
    </defs>
    <g transform="rotate(90 11 11)" clipPath="url(#icon-copy_svg__a)">
      <g
        data-name="Group 4639"
        transform="rotate(-90 53.5 280.5)"
        fill="#899ca8"
      >
        <rect
          data-name="Rectangle 2621"
          width={10}
          height={2}
          rx={1}
          transform="rotate(90 41.5 276.5)"
        />
        <rect
          data-name="Rectangle 2628"
          width={10}
          height={2}
          rx={1}
          transform="rotate(90 45.5 280.5)"
        />
        <rect
          data-name="Rectangle 2633"
          width={8}
          height={2}
          rx={1}
          transform="rotate(90 49.5 280.5)"
        />
        <rect
          data-name="Rectangle 2629"
          width={10}
          height={2}
          rx={1}
          transform="rotate(180 163 118.5)"
        />
        <rect
          data-name="Rectangle 2634"
          width={8}
          height={2}
          rx={1}
          transform="rotate(180 165 116.5)"
        />
        <rect
          data-name="Rectangle 2630"
          width={10}
          height={2}
          rx={1}
          transform="rotate(180 163 122.5)"
        />
      </g>
    </g>
  </svg>
);

export default SvgIconCopy;
