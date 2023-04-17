import * as React from 'react';
import { SVGProps } from 'react';

const SvgIconArrowDoubleGrayLeft = (props: SVGProps<SVGSVGElement>) => (
  <svg xmlns="http://www.w3.org/2000/svg" width={22} height={22} {...props}>
    <defs>
      <clipPath id="icon-arrow-double-gray-left_svg__a">
        <path data-name="Rectangle 2785" fill="#899ca8" opacity={0} d="M0 0h22v22H0z" />
      </clipPath>
    </defs>
    <g clipPath="url(#icon-arrow-double-gray-left_svg__a)" fill="#899ca8">
      <rect
        data-name="Rectangle 2781"
        width={8}
        height={2}
        rx={1}
        transform="rotate(-45 15.242 .758)"
      />
      <rect
        data-name="Rectangle 2782"
        width={8}
        height={2}
        rx={1}
        transform="rotate(-45 18.242 -6.484)"
      />
      <rect
        data-name="Rectangle 2783"
        width={8}
        height={2}
        rx={1}
        transform="rotate(-135 8.314 6.515)"
      />
      <rect
        data-name="Rectangle 2784"
        width={8}
        height={2}
        rx={1}
        transform="rotate(-135 11.314 5.272)"
      />
    </g>
  </svg>
);

export default SvgIconArrowDoubleGrayLeft;
